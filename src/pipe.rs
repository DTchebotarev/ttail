use std::io::{self, BufRead};
use std::os::unix::io::AsRawFd;
use std::sync::mpsc;
use std::thread;

use crate::event::Event;
use crate::interactive;
use crate::term;

pub fn has_tty() -> bool {
    std::fs::File::open("/dev/tty").is_ok()
}

pub fn run_pipe_mode() {
    if !has_tty() {
        interactive::run_non_interactive();
        return;
    }

    // Open /dev/tty for key reading and enable raw mode on it
    let tty = std::fs::File::open("/dev/tty").expect("failed to open /dev/tty");
    let tty_fd = tty.as_raw_fd();
    let original_termios = term::enable_raw_mode(tty_fd);

    let (tx, rx) = mpsc::channel();

    // Pipe reader thread — reads lines from stdin (the pipe)
    let tx_stdin = tx.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx_stdin.send(Event::Line(l)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        tx_stdin.send(Event::InputDone).ok();
    });

    // Key reader thread — reads raw bytes from /dev/tty
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

    interactive::run_interactive(rx, None);

    // Restore terminal mode
    term::disable_raw_mode(tty_fd, &original_termios);
}
