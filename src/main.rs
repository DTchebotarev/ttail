use std::io::{self, BufRead, BufReader, Write, stdout};
use std::os::unix::io::FromRawFd;
use std::sync::mpsc;
use std::thread;

use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use ttail::LineBuffer;

enum Event {
    Line(String),
    Key(KeyEvent),
    InputDone,
}

#[derive(PartialEq)]
enum Mode {
    Collapsed,
    Expanded,
}

fn write_line(out: &mut io::StdoutLock, line: &str, raw_mode: bool) {
    if raw_mode {
        write!(out, "{}\r\n", line).ok();
    } else {
        writeln!(out, "{}", line).ok();
    }
}

fn clear_lines(out: &mut io::StdoutLock, num_lines: usize) {
    write!(out, "\x1B[0m").ok();
    for _ in 0..num_lines {
        write!(out, "\x1B[1A\x1B[2K").ok();
    }
    out.flush().ok();
}

fn draw_collapsed(out: &mut io::StdoutLock, buf: &LineBuffer, prev_lines: usize, first: bool, raw_mode: bool, input_done: bool) -> usize {
    if !first {
        clear_lines(out, prev_lines);
    }
    let lines = buf.display_lines();
    for l in &lines {
        write_line(out, l, raw_mode);
    }
    let status = if input_done {
        format!("\x1B[2m[Tab: expand | {} lines | done]\x1B[0m", buf.total_count())
    } else {
        format!("\x1B[2m[Tab: expand | {} lines]\x1B[0m", buf.total_count())
    };
    write_line(out, &status, raw_mode);
    out.flush().ok();
    lines.len() + 1
}

fn draw_expanded(out: &mut io::StdoutLock, buf: &LineBuffer, scroll_offset: usize, viewport_height: usize, raw_mode: bool, input_done: bool) -> usize {
    let total = buf.total_count();
    let visible = viewport_height.min(total.saturating_sub(scroll_offset));
    let lines = buf.display_range(scroll_offset, viewport_height);
    for l in &lines {
        write_line(out, l, raw_mode);
    }
    let end = scroll_offset + visible;
    let at_bottom = end >= total;
    let new_indicator = if !at_bottom && total > end {
        format!(" | {} new ↓", total - end)
    } else {
        String::new()
    };
    let done_indicator = if input_done { " | done" } else { "" };
    let status = format!(
        "\x1B[2m[Tab: collapse | {}-{} of {}{}{}]\x1B[0m",
        scroll_offset + 1,
        end,
        total,
        new_indicator,
        done_indicator,
    );
    write_line(out, &status, raw_mode);
    out.flush().ok();
    visible + 1
}

fn has_tty() -> bool {
    std::fs::File::open("/dev/tty").is_ok()
}

/// Dup stdin (the pipe) and reopen stdin from /dev/tty so crossterm
/// reads key events from the terminal. Returns a BufReader over the
/// original piped stdin fd.
fn steal_stdin() -> BufReader<std::fs::File> {
    use std::os::unix::io::AsRawFd;

    let stdin_fd = io::stdin().as_raw_fd();
    let duped_fd = unsafe { libc::dup(stdin_fd) };
    assert!(duped_fd >= 0, "failed to dup stdin");

    // Reopen stdin from /dev/tty
    let tty = std::fs::File::open("/dev/tty").expect("failed to open /dev/tty");
    let tty_fd = tty.as_raw_fd();
    unsafe { libc::dup2(tty_fd, stdin_fd); }
    drop(tty);

    let pipe_file = unsafe { std::fs::File::from_raw_fd(duped_fd) };
    BufReader::new(pipe_file)
}

fn run_interactive(rx: mpsc::Receiver<Event>) {
    terminal::enable_raw_mode().expect("failed to enable raw mode");

    let stdout_handle = stdout();
    let mut out = stdout_handle.lock();
    let mut buf = LineBuffer::new(10);
    let mut mode = Mode::Collapsed;
    let mut prev_drawn_lines: usize = 0;
    let mut first = true;
    let mut input_done = false;
    let mut scroll_offset: usize = 0;

    for event in &rx {
        match event {
            Event::Line(line) => {
                let trimmed = line.trim().to_string();
                buf.push(trimmed);

                match mode {
                    Mode::Collapsed => {
                        prev_drawn_lines = draw_collapsed(&mut out, &buf, prev_drawn_lines, first, true, input_done);
                        first = false;
                    }
                    Mode::Expanded => {
                        let (_, rows) = terminal::size().unwrap_or((80, 24));
                        let viewport = (rows as usize).saturating_sub(1);
                        let total = buf.total_count();
                        let was_at_bottom = scroll_offset + viewport >= total - 1;
                        if was_at_bottom {
                            scroll_offset = total.saturating_sub(viewport);
                        }
                        clear_lines(&mut out, prev_drawn_lines);
                        prev_drawn_lines = draw_expanded(&mut out, &buf, scroll_offset, viewport, true, input_done);
                    }
                }
            }
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('q') => break,
                    KeyCode::Tab => {
                        clear_lines(&mut out, prev_drawn_lines);
                        match mode {
                            Mode::Collapsed => {
                                mode = Mode::Expanded;
                                let (_, rows) = terminal::size().unwrap_or((80, 24));
                                let viewport = (rows as usize).saturating_sub(1);
                                scroll_offset = buf.total_count().saturating_sub(viewport);
                                prev_drawn_lines = draw_expanded(&mut out, &buf, scroll_offset, viewport, true, input_done);
                            }
                            Mode::Expanded => {
                                mode = Mode::Collapsed;
                                prev_drawn_lines = draw_collapsed(&mut out, &buf, 0, true, true, input_done);
                            }
                        }
                    }
                    _ if mode == Mode::Expanded => {
                        let (_, rows) = terminal::size().unwrap_or((80, 24));
                        let viewport = (rows as usize).saturating_sub(1);
                        let total = buf.total_count();
                        let max_offset = total.saturating_sub(viewport);

                        let new_offset = match key.code {
                            KeyCode::Up | KeyCode::Char('k') => scroll_offset.saturating_sub(1),
                            KeyCode::Down | KeyCode::Char('j') => (scroll_offset + 1).min(max_offset),
                            KeyCode::PageUp => scroll_offset.saturating_sub(viewport),
                            KeyCode::PageDown => (scroll_offset + viewport).min(max_offset),
                            KeyCode::Home | KeyCode::Char('g') => 0,
                            KeyCode::End | KeyCode::Char('G') => max_offset,
                            _ => scroll_offset,
                        };

                        if new_offset != scroll_offset {
                            scroll_offset = new_offset;
                            clear_lines(&mut out, prev_drawn_lines);
                            prev_drawn_lines = draw_expanded(&mut out, &buf, scroll_offset, viewport, true, input_done);
                        }
                    }
                    _ => {}
                }
            }
            Event::InputDone => {
                input_done = true;
                clear_lines(&mut out, prev_drawn_lines);
                match mode {
                    Mode::Collapsed => {
                        prev_drawn_lines = draw_collapsed(&mut out, &buf, 0, true, true, input_done);
                    }
                    Mode::Expanded => {
                        let (_, rows) = terminal::size().unwrap_or((80, 24));
                        let viewport = (rows as usize).saturating_sub(1);
                        prev_drawn_lines = draw_expanded(&mut out, &buf, scroll_offset, viewport, true, input_done);
                    }
                }
            }
        }
    }

    drop(out);
    terminal::disable_raw_mode().ok();
    print!("\x1B[0m");
    stdout().flush().ok();
}

fn run_non_interactive() {
    let stdin = io::stdin();
    let stdout_handle = stdout();
    let mut out = stdout_handle.lock();
    let mut input = stdin.lock().lines();
    let mut buf = LineBuffer::new(10);
    let mut first = true;
    let mut prev_drawn_lines: usize = 0;

    while let Some(Ok(line)) = input.next() {
        if line.trim().is_empty() {
            break;
        }
        if !first {
            clear_lines(&mut out, prev_drawn_lines);
        }
        buf.push(line.trim().to_string());
        let lines = buf.display_lines();
        for l in &lines {
            writeln!(out, "{}", l).ok();
        }
        out.flush().ok();
        prev_drawn_lines = lines.len();
        first = false;
    }
}

fn main() {
    if !has_tty() {
        run_non_interactive();
        return;
    }

    // Dup the piped stdin and reopen stdin from /dev/tty so crossterm
    // reads key events from the terminal.
    let pipe_reader = steal_stdin();

    let (tx, rx) = mpsc::channel();

    // Stdin reader thread — reads from the original piped fd
    let tx_stdin = tx.clone();
    thread::spawn(move || {
        for line in pipe_reader.lines() {
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

    // TTY key reader thread — crossterm now reads from /dev/tty via stdin
    let tx_key = tx;
    thread::spawn(move || {
        loop {
            match event::read() {
                Ok(CEvent::Key(key_event)) => {
                    if tx_key.send(Event::Key(key_event)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    run_interactive(rx);
}
