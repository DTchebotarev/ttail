use std::io::Read;

use crate::event::{KeyCode, KeyEvent};

/// Query terminal dimensions via ioctl. Returns (cols, rows).
pub fn terminal_size() -> (u16, u16) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        (ws.ws_col, ws.ws_row)
    } else {
        (80, 24)
    }
}

/// Read a single key event from a file descriptor in raw mode.
pub fn read_key(input: &mut impl Read) -> Option<KeyEvent> {
    let mut b = [0u8; 1];
    if input.read_exact(&mut b).is_err() {
        return None;
    }

    let (code, ctrl) = match b[0] {
        0x09 => (KeyCode::Tab, false),
        0x0D => (KeyCode::Enter, false),
        0x7F => (KeyCode::Backspace, false),
        0x1B => {
            // ESC — start of CSI sequence
            if input.read_exact(&mut b).is_err() {
                return Some(KeyEvent { code: KeyCode::Esc, ctrl: false });
            }
            if b[0] != b'[' {
                return Some(KeyEvent { code: KeyCode::Esc, ctrl: false });
            }
            if input.read_exact(&mut b).is_err() {
                return Some(KeyEvent { code: KeyCode::Esc, ctrl: false });
            }
            let code = match b[0] {
                b'A' => KeyCode::Up,
                b'B' => KeyCode::Down,
                b'C' => KeyCode::Right,
                b'D' => KeyCode::Left,
                b'H' => KeyCode::Home,
                b'F' => KeyCode::End,
                b'5' | b'6' | b'3' => {
                    let kc = match b[0] {
                        b'5' => KeyCode::PageUp,
                        b'6' => KeyCode::PageDown,
                        _ => KeyCode::Delete,
                    };
                    // Consume trailing '~'
                    let _ = input.read_exact(&mut b);
                    kc
                }
                _ => KeyCode::Esc,
            };
            (code, false)
        }
        c @ 1..=26 => {
            let ch = (c - 1 + b'a') as char;
            (KeyCode::Char(ch), true)
        }
        c => (KeyCode::Char(c as char), false),
    };

    Some(KeyEvent { code, ctrl })
}

/// Enable raw mode on a file descriptor. Returns the original termios for restore.
pub fn enable_raw_mode(fd: libc::c_int) -> libc::termios {
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        libc::tcgetattr(fd, &mut t);
        let orig = t;
        libc::cfmakeraw(&mut t);
        libc::tcsetattr(fd, libc::TCSANOW, &t);
        orig
    }
}

/// Restore terminal mode from a saved termios.
pub fn disable_raw_mode(fd: libc::c_int, original: &libc::termios) {
    unsafe {
        libc::tcsetattr(fd, libc::TCSANOW, original);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::io::FromRawFd;

    fn parse(input: &[u8]) -> Option<KeyEvent> {
        let mut fds = [0i32; 2];
        unsafe {
            libc::pipe(fds.as_mut_ptr());
            libc::write(fds[1], input.as_ptr() as *const libc::c_void, input.len());
            libc::close(fds[1]);
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        read_key(&mut file)
    }

    #[test]
    fn tab_key() {
        let k = parse(&[0x09]).unwrap();
        assert_eq!(k.code, KeyCode::Tab);
        assert!(!k.ctrl);
    }

    #[test]
    fn char_q() {
        let k = parse(b"q").unwrap();
        assert_eq!(k.code, KeyCode::Char('q'));
        assert!(!k.ctrl);
    }

    #[test]
    fn ctrl_c() {
        let k = parse(&[0x03]).unwrap();
        assert_eq!(k.code, KeyCode::Char('c'));
        assert!(k.ctrl);
    }

    #[test]
    fn arrow_up() {
        let k = parse(b"\x1b[A").unwrap();
        assert_eq!(k.code, KeyCode::Up);
    }

    #[test]
    fn arrow_down() {
        let k = parse(b"\x1b[B").unwrap();
        assert_eq!(k.code, KeyCode::Down);
    }

    #[test]
    fn page_up() {
        let k = parse(b"\x1b[5~").unwrap();
        assert_eq!(k.code, KeyCode::PageUp);
    }

    #[test]
    fn page_down() {
        let k = parse(b"\x1b[6~").unwrap();
        assert_eq!(k.code, KeyCode::PageDown);
    }

    #[test]
    fn home_key() {
        let k = parse(b"\x1b[H").unwrap();
        assert_eq!(k.code, KeyCode::Home);
    }

    #[test]
    fn end_key() {
        let k = parse(b"\x1b[F").unwrap();
        assert_eq!(k.code, KeyCode::End);
    }

    #[test]
    fn enter_key() {
        let k = parse(&[0x0D]).unwrap();
        assert_eq!(k.code, KeyCode::Enter);
    }

    #[test]
    fn backspace_key() {
        let k = parse(&[0x7F]).unwrap();
        assert_eq!(k.code, KeyCode::Backspace);
    }

    #[test]
    fn delete_key() {
        let k = parse(b"\x1b[3~").unwrap();
        assert_eq!(k.code, KeyCode::Delete);
    }

    #[test]
    fn char_j() {
        let k = parse(b"j").unwrap();
        assert_eq!(k.code, KeyCode::Char('j'));
    }

    #[test]
    fn char_g_upper() {
        let k = parse(b"G").unwrap();
        assert_eq!(k.code, KeyCode::Char('G'));
    }

    #[test]
    fn eof_returns_none() {
        let mut fds = [0i32; 2];
        unsafe {
            libc::pipe(fds.as_mut_ptr());
            libc::close(fds[1]); // immediate EOF
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        assert!(read_key(&mut file).is_none());
    }
}
