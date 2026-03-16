use std::io::{self, BufRead, Write, stdout};
use std::sync::mpsc;
use std::time::Duration;

use ttail::LineBuffer;

use crate::display::{clear_lines, draw_collapsed, draw_expanded};
use crate::event::{Event, KeyCode, Mode};
use crate::pty::{self, PtyContext};
use crate::term;

fn countdown_secs() -> u8 {
    std::env::var("TTAIL_COUNTDOWN_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

/// Parse complete lines out of a byte accumulator. Leaves any trailing
/// partial line in `remainder` for the next call.
fn extract_lines(remainder: &mut Vec<u8>, data: &[u8]) -> Vec<String> {
    remainder.extend_from_slice(data);
    let mut lines = Vec::new();
    let mut start = 0;
    while let Some(pos) = remainder[start..].iter().position(|&b| b == b'\n') {
        let end = start + pos;
        let s = String::from_utf8_lossy(&remainder[start..end]);
        lines.push(s.trim_end_matches('\r').to_string());
        start = end + 1;
    }
    if start > 0 {
        remainder.drain(..start);
    }
    lines
}

fn draw_after_push(
    out: &mut io::StdoutLock,
    buf: &LineBuffer,
    mode: &Mode,
    prev_drawn_lines: &mut usize,
    first: &mut bool,
    scroll_offset: &mut usize,
    input_done: bool,
) {
    match mode {
        Mode::Collapsed => {
            *prev_drawn_lines = draw_collapsed(
                out, buf, *prev_drawn_lines, *first, true, input_done, None,
            );
            *first = false;
        }
        Mode::Expanded => {
            redraw_expanded_on_new_line(
                out, buf, scroll_offset, prev_drawn_lines, input_done,
            );
        }
    }
}

pub fn run_interactive(rx: mpsc::Receiver<Event>, pty: Option<PtyContext>) {
    let stdout_handle = stdout();
    let mut out = stdout_handle.lock();
    let mut buf = LineBuffer::new(10);
    let mut mode = Mode::Collapsed;
    let mut prev_drawn_lines: usize = 0;
    let mut first = true;
    let mut input_done = false;
    let mut scroll_offset: usize = 0;
    let mut countdown: Option<u8> = None;
    let mut pty_line_remainder: Vec<u8> = Vec::new();

    loop {
        let event = if let Some(remaining) = countdown {
            if remaining == 0 {
                break;
            }
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let new_remaining = remaining - 1;
                    countdown = Some(new_remaining);
                    if new_remaining == 0 {
                        break;
                    }
                    if mode == Mode::Collapsed {
                        clear_lines(&mut out, prev_drawn_lines);
                        prev_drawn_lines = draw_collapsed(
                            &mut out, &buf, 0, true, true, true, countdown,
                        );
                    }
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(event) => event,
                Err(_) => break,
            }
        };

        match event {
            Event::Line(line) => {
                buf.push(line.trim().to_string());
                draw_after_push(&mut out, &buf, &mode, &mut prev_drawn_lines, &mut first, &mut scroll_offset, input_done);
            }

            Event::PtyOutput(data) => {
                let new_lines = extract_lines(&mut pty_line_remainder, &data);
                if new_lines.is_empty() {
                    continue;
                }
                for line in new_lines {
                    buf.push(line.trim().to_string());
                }
                draw_after_push(&mut out, &buf, &mode, &mut prev_drawn_lines, &mut first, &mut scroll_offset, input_done);
            }

            Event::Key(key) => {
                if key.code == KeyCode::Tab {
                    countdown = None;
                    match mode {
                        Mode::Collapsed => {
                            clear_lines(&mut out, prev_drawn_lines);
                            write!(out, "\x1B[?1049h").ok();
                            out.flush().ok();
                            mode = Mode::Expanded;
                            let (_, rows) = term::terminal_size();
                            let viewport = (rows as usize).saturating_sub(1);
                            scroll_offset = buf.total_count().saturating_sub(viewport);
                            prev_drawn_lines = draw_expanded(
                                &mut out,
                                &buf,
                                scroll_offset,
                                viewport,
                                true,
                                input_done,
                            );
                        }
                        Mode::Expanded => {
                            write!(out, "\x1B[?1049l").ok();
                            out.flush().ok();
                            mode = Mode::Collapsed;
                            first = true;
                            prev_drawn_lines =
                                draw_collapsed(&mut out, &buf, 0, true, true, input_done, None);
                        }
                    }
                    continue;
                }

                // In pty collapsed mode with child alive: forward keys to child
                if let Some(ref ctx) = pty {
                    if mode == Mode::Collapsed && !input_done {
                        pty::forward_key_to_pty(ctx.master_fd, &key);
                        continue;
                    }
                }

                match key.code {
                    KeyCode::Char('c') if key.ctrl => break,
                    KeyCode::Char('q') => break,
                    _ if mode == Mode::Expanded => {
                        let (_, rows) = term::terminal_size();
                        let viewport = (rows as usize).saturating_sub(1);
                        let total = buf.total_count();
                        let max_offset = total.saturating_sub(viewport);

                        let new_offset = match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset.saturating_sub(1)
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                (scroll_offset + 1).min(max_offset)
                            }
                            KeyCode::PageUp => scroll_offset.saturating_sub(viewport),
                            KeyCode::PageDown => {
                                (scroll_offset + viewport).min(max_offset)
                            }
                            KeyCode::Home | KeyCode::Char('g') => 0,
                            KeyCode::End | KeyCode::Char('G') => max_offset,
                            _ => scroll_offset,
                        };

                        if new_offset != scroll_offset {
                            scroll_offset = new_offset;
                            clear_lines(&mut out, prev_drawn_lines);
                            prev_drawn_lines = draw_expanded(
                                &mut out,
                                &buf,
                                scroll_offset,
                                viewport,
                                true,
                                input_done,
                            );
                        }
                    }
                    _ => {}
                }
            }

            Event::InputDone => {
                input_done = true;

                countdown = Some(countdown_secs());

                if mode == Mode::Collapsed {
                    if prev_drawn_lines > 0 {
                        clear_lines(&mut out, prev_drawn_lines);
                    }
                    prev_drawn_lines = draw_collapsed(
                        &mut out, &buf, 0, true, true, input_done, countdown,
                    );
                } else {
                    clear_lines(&mut out, prev_drawn_lines);
                    let (_, rows) = term::terminal_size();
                    let viewport = (rows as usize).saturating_sub(1);
                    prev_drawn_lines = draw_expanded(
                        &mut out,
                        &buf,
                        scroll_offset,
                        viewport,
                        true,
                        input_done,
                    );
                }
            }
        }
    }

    if mode == Mode::Expanded {
        write!(out, "\x1B[?1049l").ok();
        out.flush().ok();
    }
    drop(out);
    print!("\x1B[0m");
    stdout().flush().ok();
}

fn redraw_expanded_on_new_line(
    out: &mut io::StdoutLock,
    buf: &LineBuffer,
    scroll_offset: &mut usize,
    prev_drawn_lines: &mut usize,
    input_done: bool,
) {
    let (_, rows) = term::terminal_size();
    let viewport = (rows as usize).saturating_sub(1);
    let total = buf.total_count();
    let was_at_bottom = total > 0 && *scroll_offset + viewport >= total - 1;
    if was_at_bottom {
        *scroll_offset = total.saturating_sub(viewport);
    }
    clear_lines(out, *prev_drawn_lines);
    *prev_drawn_lines =
        draw_expanded(out, buf, *scroll_offset, viewport, true, input_done);
}

pub fn run_non_interactive() {
    let stdin = io::stdin();
    let stdout_handle = stdout();
    let mut out = stdout_handle.lock();
    let mut input = stdin.lock().lines();
    let mut buf = LineBuffer::new(10);
    let mut first = true;
    let mut prev_drawn_lines: usize = 0;

    while let Some(Ok(line)) = input.next() {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            break;
        }
        if !first {
            clear_lines(&mut out, prev_drawn_lines);
        }
        buf.push(trimmed);
        let lines = buf.display_lines();
        for l in &lines {
            writeln!(out, "{}", l).ok();
        }
        out.flush().ok();
        prev_drawn_lines = lines.len();
        first = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_lines_complete() {
        let mut remainder = Vec::new();
        let lines = extract_lines(&mut remainder, b"hello\nworld\n");
        assert_eq!(lines, &["hello", "world"]);
        assert!(remainder.is_empty());
    }

    #[test]
    fn extract_lines_partial() {
        let mut remainder = Vec::new();
        let lines = extract_lines(&mut remainder, b"hel");
        assert!(lines.is_empty());
        assert_eq!(remainder, b"hel");

        let lines = extract_lines(&mut remainder, b"lo\nworld\npar");
        assert_eq!(lines, &["hello", "world"]);
        assert_eq!(remainder, b"par");
    }

    #[test]
    fn extract_lines_strips_cr() {
        let mut remainder = Vec::new();
        let lines = extract_lines(&mut remainder, b"hello\r\nworld\r\n");
        assert_eq!(lines, &["hello", "world"]);
    }

    #[test]
    fn extract_lines_empty_line() {
        let mut remainder = Vec::new();
        let lines = extract_lines(&mut remainder, b"\n\n");
        assert_eq!(lines, &["", ""]);
    }

    #[test]
    fn extract_lines_no_newline() {
        let mut remainder = Vec::new();
        let lines = extract_lines(&mut remainder, b"no newline here");
        assert!(lines.is_empty());
        assert_eq!(remainder.len(), 15);
    }
}
