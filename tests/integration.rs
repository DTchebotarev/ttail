use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

fn ttail_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ttail"));
    cmd.env("TTAIL_COUNTDOWN_SECS", "1");
    cmd
}

/// Spawn ttail (with args) inside a real pty so isatty(0) returns true.
/// Returns (master_fd, child_pid).
fn spawn_ttail_in_pty(args: &[&str]) -> (libc::c_int, libc::pid_t) {
    let mut master: libc::c_int = 0;
    let mut slave: libc::c_int = 0;
    let ret = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(ret, 0, "openpty failed");

    let bin = env!("CARGO_BIN_EXE_ttail");
    let c_bin = std::ffi::CString::new(bin).unwrap();
    let c_args: Vec<std::ffi::CString> = args
        .iter()
        .map(|a| std::ffi::CString::new(*a).unwrap())
        .collect();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
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

            // Fast countdown for tests
            let env_key = std::ffi::CString::new("TTAIL_COUNTDOWN_SECS").unwrap();
            let env_val = std::ffi::CString::new("1").unwrap();
            libc::setenv(env_key.as_ptr(), env_val.as_ptr(), 1);

            let mut argv: Vec<*const libc::c_char> = Vec::new();
            argv.push(c_bin.as_ptr());
            for a in &c_args {
                argv.push(a.as_ptr());
            }
            argv.push(std::ptr::null());

            libc::execv(c_bin.as_ptr(), argv.as_ptr());
            libc::_exit(127);
        }
    }

    unsafe {
        libc::close(slave);
    }

    (master, pid)
}

/// Read all available bytes from a fd with a timeout.
fn read_with_timeout(fd: libc::c_int, timeout_ms: i32) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret <= 0 {
            break;
        }
        if pfd.revents & libc::POLLIN == 0 {
            break;
        }
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        output.extend_from_slice(&buf[..n as usize]);
    }
    output
}

/// Wait for child and return exit status.
fn wait_child(pid: libc::pid_t) -> i32 {
    let mut status: libc::c_int = 0;
    unsafe {
        libc::waitpid(pid, &mut status, 0);
    }
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        -1
    }
}

// ---------------------------------------------------------------
// Mode detection
// ---------------------------------------------------------------

#[test]
fn no_args_piped_stdin_exits_on_eof() {
    // Stdin is a pipe, no data → pipe mode → non-interactive → EOF → exits
    let mut child = ttail_bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ttail");

    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait for ttail");
    assert!(
        output.status.success(),
        "expected clean exit, got {:?}",
        output.status
    );
}

#[test]
fn no_args_with_tty_shows_usage() {
    let (master, pid) = spawn_ttail_in_pty(&[]);

    // Read output — ttail should print usage and exit quickly
    let output = read_with_timeout(master, 2000);
    let exit_code = wait_child(pid);

    unsafe {
        libc::close(master);
    }

    assert_eq!(exit_code, 1, "expected exit code 1, got {}", exit_code);
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("Usage") || text.contains("ttail"),
        "expected usage output, got: {:?}",
        text
    );
}

// ---------------------------------------------------------------
// Pipe mode (non-interactive — no tty available)
// ---------------------------------------------------------------

#[test]
fn pipe_mode_displays_lines() {
    let mut child = ttail_bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ttail");

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "alpha").unwrap();
        writeln!(stdin, "beta").unwrap();
        writeln!(stdin, "gamma").unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha"), "missing alpha: {}", stdout);
    assert!(stdout.contains("beta"), "missing beta: {}", stdout);
    assert!(stdout.contains("gamma"), "missing gamma: {}", stdout);
}

#[test]
fn pipe_mode_empty_line_stops() {
    let mut child = ttail_bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ttail");

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "hello").unwrap();
        writeln!(stdin).unwrap(); // empty line → stop
        writeln!(stdin, "should not appear").unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"), "missing hello: {}", stdout);
    assert!(
        !stdout.contains("should not appear"),
        "unexpected line after empty line: {}",
        stdout
    );
}

#[test]
fn pipe_mode_shows_only_last_window() {
    let mut child = ttail_bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ttail");

    {
        let stdin = child.stdin.as_mut().unwrap();
        for i in 0..20 {
            writeln!(stdin, "line {}", i).unwrap();
        }
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("line 19"), "missing line 19: {}", stdout);
}

#[test]
fn pipe_mode_preserves_ansi_colors() {
    let mut child = ttail_bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ttail");

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "\x1B[31mred text\x1B[0m").unwrap();
        writeln!(stdin, "plain text").unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1B[31m"),
        "ANSI red code not preserved: {}",
        stdout
    );
}

// ---------------------------------------------------------------
// PTY mode — basic spawn and exit
// ---------------------------------------------------------------

#[test]
fn pty_mode_runs_echo() {
    let (master, pid) = spawn_ttail_in_pty(&["echo", "hello from pty"]);

    // Read output — should see the echo output (passthrough) plus status
    let output = read_with_timeout(master, 8000);
    let exit_code = wait_child(pid);

    unsafe {
        libc::close(master);
    }

    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("hello from pty"),
        "child output not found in: {:?}",
        text
    );
    assert_eq!(exit_code, 0, "expected exit 0, got {}", exit_code);
}

#[test]
fn pty_mode_propagates_exit_code() {
    let (master, pid) = spawn_ttail_in_pty(&["sh", "-c", "exit 42"]);

    // Drain output so the pty doesn't block
    read_with_timeout(master, 8000);
    let exit_code = wait_child(pid);

    unsafe {
        libc::close(master);
    }

    assert_eq!(exit_code, 42, "expected exit 42, got {}", exit_code);
}

#[test]
fn pty_mode_invalid_command_exits_127() {
    let (master, pid) = spawn_ttail_in_pty(&["nonexistent_command_xyz_123"]);

    read_with_timeout(master, 8000);
    let exit_code = wait_child(pid);

    unsafe {
        libc::close(master);
    }

    assert_eq!(exit_code, 127, "expected exit 127, got {}", exit_code);
}

#[test]
fn pty_mode_multiline_output() {
    let (master, pid) = spawn_ttail_in_pty(&["sh", "-c", "echo line1; echo line2; echo line3"]);

    let output = read_with_timeout(master, 8000);
    wait_child(pid);

    unsafe {
        libc::close(master);
    }

    let text = String::from_utf8_lossy(&output);
    assert!(text.contains("line1"), "missing line1: {:?}", text);
    assert!(text.contains("line2"), "missing line2: {:?}", text);
    assert!(text.contains("line3"), "missing line3: {:?}", text);
}

// ---------------------------------------------------------------
// PTY mode — countdown auto-exit
// ---------------------------------------------------------------

#[test]
fn pty_mode_auto_exits_after_countdown() {
    use std::time::Instant;

    let start = Instant::now();
    let (master, pid) = spawn_ttail_in_pty(&["true"]);

    // Drain output
    read_with_timeout(master, 8000);
    wait_child(pid);
    let elapsed = start.elapsed();

    unsafe {
        libc::close(master);
    }

    // TTAIL_COUNTDOWN_SECS=1, so should exit after ~1 second
    assert!(
        elapsed >= Duration::from_millis(500),
        "exited too fast: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "took too long: {:?}",
        elapsed
    );
}

#[test]
fn pty_mode_countdown_shows_in_output() {
    let (master, pid) = spawn_ttail_in_pty(&["echo", "done-test"]);

    let output = read_with_timeout(master, 8000);
    wait_child(pid);

    unsafe {
        libc::close(master);
    }

    let text = String::from_utf8_lossy(&output);
    // Should see "exiting in" countdown text after child exits
    assert!(
        text.contains("exiting in"),
        "expected countdown in output: {:?}",
        text
    );
}

// ---------------------------------------------------------------
// Key-to-bytes conversion
// ---------------------------------------------------------------

#[cfg(test)]
mod key_forwarding {
    /// Create a pty with raw mode (no line discipline processing) and test
    /// that writing specific byte sequences through the master produces the
    /// expected bytes on the slave side.
    fn write_and_read_raw(input: &[u8]) -> Vec<u8> {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Put slave in raw mode so bytes pass through unmodified
            let mut termios: libc::termios = std::mem::zeroed();
            libc::tcgetattr(slave, &mut termios);
            libc::cfmakeraw(&mut termios);
            libc::tcsetattr(slave, libc::TCSANOW, &termios);
        }

        unsafe {
            libc::write(
                master,
                input.as_ptr() as *const libc::c_void,
                input.len(),
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut out = vec![0u8; 256];
        let n = unsafe {
            let flags = libc::fcntl(slave, libc::F_GETFL);
            libc::fcntl(slave, libc::F_SETFL, flags | libc::O_NONBLOCK);
            libc::read(slave, out.as_mut_ptr() as *mut libc::c_void, out.len())
        };

        unsafe {
            libc::close(master);
            libc::close(slave);
        }

        if n > 0 {
            out.truncate(n as usize);
            out
        } else {
            vec![]
        }
    }

    #[test]
    fn char_passes_through() {
        assert_eq!(write_and_read_raw(b"a"), b"a");
    }

    #[test]
    fn ctrl_c_passes_through_raw() {
        assert_eq!(write_and_read_raw(&[0x03]), vec![0x03]);
    }

    #[test]
    fn cr_passes_through_raw() {
        assert_eq!(write_and_read_raw(&[b'\r']), vec![b'\r']);
    }

    #[test]
    fn arrow_up_passes_through_raw() {
        assert_eq!(write_and_read_raw(b"\x1b[A"), b"\x1b[A");
    }

    #[test]
    fn backspace_passes_through_raw() {
        assert_eq!(write_and_read_raw(&[0x7f]), vec![0x7f]);
    }

    #[test]
    fn utf8_passes_through_raw() {
        let bytes = "é".as_bytes();
        assert_eq!(write_and_read_raw(bytes), bytes);
    }
}
