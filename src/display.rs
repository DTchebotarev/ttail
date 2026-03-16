use std::io::{self, Write};

use ttail::LineBuffer;

pub fn write_line(out: &mut io::StdoutLock, line: &str, raw_mode: bool) {
    if raw_mode {
        write!(out, "{}\r\n", line).ok();
    } else {
        writeln!(out, "{}", line).ok();
    }
}

pub fn clear_lines(out: &mut io::StdoutLock, num_lines: usize) {
    write!(out, "\x1B[0m").ok();
    for _ in 0..num_lines {
        write!(out, "\x1B[1A\x1B[2K").ok();
    }
    out.flush().ok();
}

pub fn draw_collapsed(
    out: &mut io::StdoutLock,
    buf: &LineBuffer,
    prev_lines: usize,
    first: bool,
    raw_mode: bool,
    input_done: bool,
) -> usize {
    if !first {
        clear_lines(out, prev_lines);
    }
    let lines = buf.display_lines();
    for l in &lines {
        write_line(out, l, raw_mode);
    }
    let status = if input_done {
        format!(
            "\x1B[0;2m[Tab: expand | {} lines | done]\x1B[0m",
            buf.total_count()
        )
    } else {
        format!("\x1B[0;2m[Tab: expand | {} lines]\x1B[0m", buf.total_count())
    };
    write_line(out, &status, raw_mode);
    out.flush().ok();
    lines.len() + 1
}

pub fn draw_expanded(
    out: &mut io::StdoutLock,
    buf: &LineBuffer,
    scroll_offset: usize,
    viewport_height: usize,
    raw_mode: bool,
    input_done: bool,
) -> usize {
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
        "\x1B[0;2m[Tab: collapse | {}-{} of {}{}{}]\x1B[0m",
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
