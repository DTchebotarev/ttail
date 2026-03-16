use std::ffi::CString;
use std::io::{self, Read};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::mpsc;
use std::thread;

use signal_hook::iterator::Signals;

use crate::event::{Event, KeyCode, KeyEvent};
use crate::interactive;
use crate::term;

/// Context passed to the interactive loop for pty-specific behavior.
pub struct PtyContext {
    pub master_fd: RawFd,
}

fn open_pty() -> io::Result<(RawFd, RawFd)> {
    let mut master: RawFd = 0;
    let mut slave: RawFd = 0;
    let ret = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((master, slave))
}

fn set_pty_size(fd: RawFd, cols: u16, rows: u16) {
    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        libc::ioctl(fd, libc::TIOCSWINSZ, &ws);
    }
}

struct PtyChild {
    master_fd: RawFd,
    child_pid: libc::pid_t,
}

fn spawn_in_pty(cmd: &str, args: &[String]) -> io::Result<PtyChild> {
    let (master, slave) = open_pty()?;

    let (cols, rows) = term::terminal_size();
    set_pty_size(slave, cols, rows);

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if pid == 0 {
        // Child process
        unsafe {
            libc::setsid();
            libc::ioctl(slave, libc::TIOCSCTTY as libc::c_ulong, 0);
            libc::dup2(slave, 0);
            libc::dup2(slave, 1);
            libc::dup2(slave, 2);
            if slave > 2 {
                libc::close(slave);
            }
            libc::close(master);

            let c_cmd = CString::new(cmd).unwrap();
            let mut c_args: Vec<CString> = Vec::new();
            c_args.push(c_cmd.clone());
            for a in args {
                c_args.push(CString::new(a.as_str()).unwrap());
            }
            let c_argv: Vec<*const libc::c_char> = c_args
                .iter()
                .map(|a| a.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect();

            libc::execvp(c_cmd.as_ptr(), c_argv.as_ptr());
            libc::_exit(127);
        }
    }

    unsafe {
        libc::close(slave);
    }

    Ok(PtyChild {
        master_fd: master,
        child_pid: pid,
    })
}

fn spawn_pty_reader(master_fd: RawFd, tx: mpsc::Sender<Event>) {
    let reader_fd = unsafe { libc::dup(master_fd) };
    assert!(reader_fd >= 0, "failed to dup master fd");

    thread::spawn(move || {
        let mut file = unsafe { std::fs::File::from_raw_fd(reader_fd) };
        let mut buf = [0u8; 4096];
        loop {
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(Event::PtyOutput(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    if e.raw_os_error() == Some(libc::EIO) {
                        break;
                    }
                    break;
                }
            }
        }
        tx.send(Event::InputDone).ok();
    });
}

fn spawn_sigwinch_handler(master_fd: RawFd) {
    thread::spawn(move || {
        let mut signals = Signals::new([signal_hook::consts::SIGWINCH])
            .expect("failed to register SIGWINCH handler");
        for _ in signals.forever() {
            let (cols, rows) = term::terminal_size();
            set_pty_size(master_fd, cols, rows);
        }
    });
}

fn wait_for_child(pid: libc::pid_t) -> i32 {
    let mut status: libc::c_int = 0;
    unsafe {
        libc::waitpid(pid, &mut status, 0);
    }
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        1
    }
}

/// Convert a KeyEvent into bytes and write to the pty master fd.
pub fn forward_key_to_pty(master_fd: RawFd, key: &KeyEvent) {
    let bytes: Vec<u8> = match key.code {
        KeyCode::Char(c) => {
            if key.ctrl {
                if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
                    let ctrl = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                    vec![ctrl]
                } else {
                    return;
                }
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Tab => vec![0x09],
    };

    unsafe {
        libc::write(
            master_fd,
            bytes.as_ptr() as *const libc::c_void,
            bytes.len(),
        );
    }
}

pub fn run_pty_mode(cmd: &str, args: &[String]) {
    let child = spawn_in_pty(cmd, args).unwrap_or_else(|e| {
        eprintln!("ttail: failed to spawn '{}': {}", cmd, e);
        std::process::exit(127);
    });
    let master_fd = child.master_fd;
    let child_pid = child.child_pid;

    spawn_sigwinch_handler(master_fd);

    // Open /dev/tty for key reading and enable raw mode
    let tty = std::fs::File::open("/dev/tty").expect("failed to open /dev/tty");
    let tty_fd = tty.as_raw_fd();
    let original_termios = term::enable_raw_mode(tty_fd);

    let (tx, rx) = mpsc::channel();

    spawn_pty_reader(master_fd, tx.clone());

    // Key reader thread — reads from /dev/tty
    let tx_key = tx;
    let mut tty_reader = tty;
    thread::spawn(move || loop {
        match term::read_key(&mut tty_reader) {
            Some(key) => {
                if tx_key.send(Event::Key(key)).is_err() {
                    break;
                }
            }
            None => break,
        }
    });

    let pty_ctx = PtyContext { master_fd };
    interactive::run_interactive(rx, Some(pty_ctx));

    // Restore terminal
    term::disable_raw_mode(tty_fd, &original_termios);

    // Close master fd — sends SIGHUP to child's process group
    unsafe {
        libc::close(master_fd);
    }

    let exit_code = wait_for_child(child_pid);
    std::process::exit(exit_code);
}
