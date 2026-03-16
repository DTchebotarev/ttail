use std::collections::VecDeque;
use std::fmt::Write;
use std::io::Write as IoWrite;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Default, PartialEq, Debug)]
pub struct AnsiState {
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg: Option<Color>,
    bg: Option<Color>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Color {
    Basic(u8),       // 30-37, 39, 40-47, 49, 90-97, 100-107
    Palette(u8),     // 38;5;N or 48;5;N
    Rgb(u8, u8, u8), // 38;2;R;G;B or 48;2;R;G;B
}

impl AnsiState {
    fn apply_sgr(&mut self, params: &[u8]) {
        if params.is_empty() {
            *self = Self::default();
            return;
        }
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => *self = Self::default(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                5 => self.blink = true,
                7 => self.reverse = true,
                8 => self.hidden = true,
                9 => self.strikethrough = true,
                22 => { self.bold = false; self.dim = false; }
                23 => self.italic = false,
                24 => self.underline = false,
                25 => self.blink = false,
                27 => self.reverse = false,
                28 => self.hidden = false,
                29 => self.strikethrough = false,
                n @ 30..=37 => self.fg = Some(Color::Basic(n)),
                38 => {
                    if let Some(color) = Self::parse_extended(&params[i..]) {
                        match color {
                            Color::Palette(_) => i += 2,
                            Color::Rgb(_, _, _) => i += 4,
                            _ => {}
                        }
                        self.fg = Some(color);
                    }
                }
                39 => self.fg = None,
                n @ 40..=47 => self.bg = Some(Color::Basic(n)),
                48 => {
                    if let Some(color) = Self::parse_extended(&params[i..]) {
                        match color {
                            Color::Palette(_) => i += 2,
                            Color::Rgb(_, _, _) => i += 4,
                            _ => {}
                        }
                        self.bg = Some(color);
                    }
                }
                49 => self.bg = None,
                n @ 90..=97 => self.fg = Some(Color::Basic(n)),
                n @ 100..=107 => self.bg = Some(Color::Basic(n)),
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_extended(params: &[u8]) -> Option<Color> {
        if params.len() >= 3 && params[1] == 5 {
            Some(Color::Palette(params[2]))
        } else if params.len() >= 5 && params[1] == 2 {
            Some(Color::Rgb(params[2], params[3], params[4]))
        } else {
            None
        }
    }

    pub fn to_escape(&self) -> String {
        let mut out = String::new();
        let mut sep = false;

        macro_rules! push_code {
            ($val:expr) => {{
                if sep { out.push(';'); }
                write!(out, "{}", $val).unwrap();
                sep = true;
            }};
        }

        if self.bold { push_code!("1"); }
        if self.dim { push_code!("2"); }
        if self.italic { push_code!("3"); }
        if self.underline { push_code!("4"); }
        if self.blink { push_code!("5"); }
        if self.reverse { push_code!("7"); }
        if self.hidden { push_code!("8"); }
        if self.strikethrough { push_code!("9"); }
        if let Some(ref c) = self.fg {
            Self::write_color(&mut out, c, 38, &mut sep);
        }
        if let Some(ref c) = self.bg {
            Self::write_color(&mut out, c, 48, &mut sep);
        }
        if out.is_empty() {
            String::new()
        } else {
            format!("\x1B[{out}m")
        }
    }

    fn write_color(out: &mut String, color: &Color, prefix: u8, sep: &mut bool) {
        if *sep { out.push(';'); }
        match color {
            Color::Basic(n) => write!(out, "{n}").unwrap(),
            Color::Palette(n) => write!(out, "{prefix};5;{n}").unwrap(),
            Color::Rgb(r, g, b) => write!(out, "{prefix};2;{r};{g};{b}").unwrap(),
        }
        *sep = true;
    }

    pub fn is_empty(&self) -> bool {
        !self.bold && !self.dim && !self.italic && !self.underline
            && !self.blink && !self.reverse && !self.hidden && !self.strikethrough
            && self.fg.is_none() && self.bg.is_none()
    }
}

fn parse_sgr_params(seq: &str) -> ([u8; 16], usize) {
    if seq.is_empty() {
        let mut arr = [0u8; 16];
        arr[0] = 0;
        return (arr, 1);
    }
    let mut arr = [0u8; 16];
    let mut len = 0;
    for s in seq.split(';') {
        if len >= 16 { break; }
        if let Ok(v) = s.parse::<u8>() {
            arr[len] = v;
            len += 1;
        }
    }
    (arr, len)
}

pub fn update_ansi_state(state: &mut AnsiState, line: &str) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == 0x1B && i + 1 < len && bytes[i + 1] == b'[' {
            i += 2;
            let start = i;
            while i < len && bytes[i] != b'm' && bytes[i] != b'A' && bytes[i] != b'B'
                && bytes[i] != b'C' && bytes[i] != b'D' && bytes[i] != b'H'
                && bytes[i] != b'J' && bytes[i] != b'K'
            {
                i += 1;
            }
            if i < len && bytes[i] == b'm' {
                let param_str = &line[start..i];
                let (params, len) = parse_sgr_params(param_str);
                state.apply_sgr(&params[..len]);
            }
            i += 1;
        } else {
            i += 1;
        }
    }
}

pub struct LineBuffer {
    lines: VecDeque<String>,
    window_size: usize,
    max_history: usize,
    ansi_prefix: AnsiState,
    spill_file: Option<std::fs::File>,
    spill_path: Option<std::path::PathBuf>,
    disk_line_count: usize,
}

impl Drop for LineBuffer {
    fn drop(&mut self) {
        if let Some(ref path) = self.spill_path {
            std::fs::remove_file(path).ok();
        }
    }
}

impl LineBuffer {
    pub fn new(window_size: usize) -> Self {
        Self::with_max_history(window_size, 10_000)
    }

    pub fn with_max_history(window_size: usize, max_history: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(window_size.min(max_history)),
            window_size,
            max_history,
            ansi_prefix: AnsiState::default(),
            spill_file: None,
            spill_path: None,
            disk_line_count: 0,
        }
    }

    pub fn push(&mut self, line: String) {
        self.lines.push_back(line);
        let total = self.lines.len();
        if total > self.window_size {
            let idx = total - self.window_size - 1;
            update_ansi_state(&mut self.ansi_prefix, &self.lines[idx]);
        }
        if self.lines.len() > self.max_history {
            let evicted = self.lines.pop_front().unwrap();
            self.spill_to_disk(&evicted);
        }
    }

    fn spill_to_disk(&mut self, line: &str) {
        if self.spill_file.is_none() {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("ttail-{}-{}.log", std::process::id(), id));
            let file = std::fs::File::create(&path).expect("failed to create spill file");
            self.spill_path = Some(path);
            self.spill_file = Some(file);
        }
        if let Some(ref mut file) = self.spill_file {
            writeln!(file, "{}", line).ok();
        }
        self.disk_line_count += 1;
    }

    fn read_disk_lines(&self, start: usize, count: usize) -> Vec<String> {
        use std::io::{BufRead, BufReader};
        let Some(ref path) = self.spill_path else {
            return Vec::new();
        };
        let Ok(file) = std::fs::File::open(path) else {
            return Vec::new();
        };
        BufReader::new(file)
            .lines()
            .skip(start)
            .take(count)
            .filter_map(|l| l.ok())
            .collect()
    }

    pub fn visible_len(&self) -> usize {
        self.lines.len().min(self.window_size)
    }

    pub fn total_count(&self) -> usize {
        self.disk_line_count + self.lines.len()
    }

    pub fn all_lines(&self) -> &VecDeque<String> {
        &self.lines
    }

    pub fn window_lines(&self) -> Vec<&str> {
        let total = self.lines.len();
        let start = total.saturating_sub(self.window_size);
        self.lines.range(start..).map(|s| s.as_str()).collect()
    }

    pub fn display_lines(&self) -> Vec<String> {
        let total = self.lines.len();
        let start = total.saturating_sub(self.window_size);
        if self.ansi_prefix.is_empty() {
            return self.lines.range(start..).map(|s| s.to_string()).collect();
        }
        let prefix = self.ansi_prefix.to_escape();
        let mut result: Vec<String> = Vec::with_capacity(self.visible_len());
        let mut first = true;
        for line in self.lines.range(start..) {
            if first {
                result.push(format!("{prefix}{line}"));
                first = false;
            } else {
                result.push(line.to_string());
            }
        }
        result
    }

    pub fn display_range(&self, start: usize, count: usize) -> Vec<String> {
        let total = self.total_count();
        let end = (start + count).min(total);
        if start >= total {
            return Vec::new();
        }

        // Compute ANSI state by scanning all lines before `start`
        let mut state = AnsiState::default();
        if self.disk_line_count > 0 {
            let disk_scan_end = start.min(self.disk_line_count);
            if disk_scan_end > 0 {
                for line in self.read_disk_lines(0, disk_scan_end) {
                    update_ansi_state(&mut state, &line);
                }
            }
        }
        if start > self.disk_line_count {
            let mem_scan_end = start - self.disk_line_count;
            for i in 0..mem_scan_end {
                update_ansi_state(&mut state, &self.lines[i]);
            }
        }

        // Collect lines in the display range from disk and/or memory
        let mut raw_lines: Vec<String> = Vec::with_capacity(end - start);
        if start < self.disk_line_count {
            let disk_end = end.min(self.disk_line_count);
            raw_lines.extend(self.read_disk_lines(start, disk_end - start));
        }
        if end > self.disk_line_count {
            let mem_start = start.saturating_sub(self.disk_line_count);
            let mem_end = end - self.disk_line_count;
            for line in self.lines.range(mem_start..mem_end) {
                raw_lines.push(line.to_string());
            }
        }

        if raw_lines.is_empty() {
            return Vec::new();
        }
        if state.is_empty() {
            return raw_lines;
        }
        let prefix = state.to_escape();
        raw_lines[0] = format!("{prefix}{}", raw_lines[0]);
        raw_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_stays_within_window() {
        let mut buf = LineBuffer::new(3);
        for i in 0..5 {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.visible_len(), 3);
        assert_eq!(buf.total_count(), 5);
        assert_eq!(buf.window_lines(), &["line 2", "line 3", "line 4"]);
    }

    #[test]
    fn buffer_under_window() {
        let mut buf = LineBuffer::new(10);
        buf.push("a".to_string());
        buf.push("b".to_string());
        assert_eq!(buf.visible_len(), 2);
        assert_eq!(buf.total_count(), 2);
        assert_eq!(buf.window_lines(), &["a", "b"]);
    }

    #[test]
    fn buffer_exact_window() {
        let mut buf = LineBuffer::new(3);
        buf.push("a".to_string());
        buf.push("b".to_string());
        buf.push("c".to_string());
        assert_eq!(buf.visible_len(), 3);
        assert_eq!(buf.window_lines(), &["a", "b", "c"]);
    }

    #[test]
    fn window_shows_latest() {
        let mut buf = LineBuffer::new(2);
        buf.push("first".to_string());
        buf.push("second".to_string());
        buf.push("third".to_string());
        assert_eq!(buf.window_lines(), &["second", "third"]);
    }

    #[test]
    fn buffer_window_one() {
        let mut buf = LineBuffer::new(1);
        buf.push("a".to_string());
        buf.push("b".to_string());
        assert_eq!(buf.visible_len(), 1);
        assert_eq!(buf.window_lines(), &["b"]);
    }

    #[test]
    fn empty_buffer() {
        let buf = LineBuffer::new(10);
        assert_eq!(buf.visible_len(), 0);
        assert_eq!(buf.total_count(), 0);
        assert!(buf.window_lines().is_empty());
    }

    #[test]
    fn all_lines_retained() {
        let mut buf = LineBuffer::new(3);
        for i in 0..100 {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.total_count(), 100);
        assert_eq!(buf.visible_len(), 3);
        assert_eq!(buf.all_lines()[0], "line 0");
        assert_eq!(buf.all_lines()[99], "line 99");
    }

    // ANSI state tracking tests

    #[test]
    fn no_ansi_no_prefix() {
        let mut buf = LineBuffer::new(2);
        buf.push("plain line 1".to_string());
        buf.push("plain line 2".to_string());
        buf.push("plain line 3".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["plain line 2", "plain line 3"]);
    }

    #[test]
    fn color_from_evicted_line_carries_forward() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[31mred text".to_string());
        buf.push("still red".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["\x1B[31mstill red"]);
    }

    #[test]
    fn reset_in_evicted_line_clears_state() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[31mred\x1B[0m".to_string());
        buf.push("should be plain".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["should be plain"]);
    }

    #[test]
    fn bold_carries_forward() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[1mbold text".to_string());
        buf.push("still bold".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["\x1B[1mstill bold"]);
    }

    #[test]
    fn multiple_attributes_carry_forward() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[1;31mbold red".to_string());
        buf.push("still bold red".to_string());
        let display = buf.display_lines();
        let display_line = &display[0];
        assert!(display_line.contains("\x1B["));
        assert!(display_line.contains("1"));
        assert!(display_line.contains("31"));
        assert!(display_line.ends_with("still bold red"));
    }

    #[test]
    fn color_within_buffer_no_prefix() {
        let mut buf = LineBuffer::new(3);
        buf.push("plain".to_string());
        buf.push("\x1B[32mgreen".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["plain", "\x1B[32mgreen"]);
    }

    #[test]
    fn bg_color_carries_forward() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[44mblue bg".to_string());
        buf.push("still blue bg".to_string());
        let display = buf.display_lines();
        assert_eq!(display, &["\x1B[44mstill blue bg"]);
    }

    #[test]
    fn palette_color_carries_forward() {
        let mut buf = LineBuffer::new(1);
        buf.push("\x1B[38;5;208morange".to_string());
        buf.push("still orange".to_string());
        let display = buf.display_lines();
        let line = &display[0];
        assert!(line.starts_with("\x1B["));
        assert!(line.ends_with("still orange"));
    }

    #[test]
    fn display_lines_only_prefixes_first_line() {
        let mut buf = LineBuffer::new(3);
        buf.push("\x1B[31mred".to_string());
        buf.push("line 2".to_string());
        buf.push("line 3".to_string());
        buf.push("line 4".to_string());
        let display = buf.display_lines();
        assert!(display[0].starts_with("\x1B[31m"));
        assert_eq!(display[1], "line 3");
        assert_eq!(display[2], "line 4");
    }

    #[test]
    fn update_ansi_state_basic_fg() {
        let mut state = AnsiState::default();
        update_ansi_state(&mut state, "\x1B[31mhello");
        assert_eq!(state.fg, Some(Color::Basic(31)));
    }

    #[test]
    fn update_ansi_state_reset() {
        let mut state = AnsiState::default();
        update_ansi_state(&mut state, "\x1B[31mhello");
        update_ansi_state(&mut state, "\x1B[0mworld");
        assert!(state.is_empty());
    }

    #[test]
    fn display_range_with_ansi() {
        let mut buf = LineBuffer::new(2);
        buf.push("\x1B[31mred".to_string());
        buf.push("line 2".to_string());
        buf.push("line 3".to_string());
        let range = buf.display_range(1, 2);
        assert!(range[0].starts_with("\x1B[31m"));
        assert_eq!(range[0].ends_with("line 2"), true);
        assert_eq!(range[1], "line 3");
    }

    #[test]
    fn display_range_from_start() {
        let mut buf = LineBuffer::new(2);
        buf.push("line 0".to_string());
        buf.push("line 1".to_string());
        buf.push("line 2".to_string());
        let range = buf.display_range(0, 2);
        assert_eq!(range, &["line 0", "line 1"]);
    }

    #[test]
    fn display_range_clamps_to_total() {
        let mut buf = LineBuffer::new(10);
        buf.push("a".to_string());
        buf.push("b".to_string());
        let range = buf.display_range(0, 100);
        assert_eq!(range, &["a", "b"]);
    }

    #[test]
    fn max_history_caps_memory() {
        let mut buf = LineBuffer::with_max_history(3, 10);
        for i in 0..20 {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.total_count(), 20);
        assert_eq!(buf.lines.len(), 10);
        assert_eq!(buf.disk_line_count, 10);
        assert_eq!(buf.visible_len(), 3);
        assert_eq!(buf.window_lines(), &["line 17", "line 18", "line 19"]);
    }

    #[test]
    fn disk_spill_lines_readable() {
        let mut buf = LineBuffer::with_max_history(2, 5);
        for i in 0..10 {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.disk_line_count, 5);
        assert_eq!(buf.lines.len(), 5);
        let range = buf.display_range(0, 3);
        assert_eq!(range, &["line 0", "line 1", "line 2"]);
        let range = buf.display_range(3, 4);
        assert_eq!(range, &["line 3", "line 4", "line 5", "line 6"]);
        let range = buf.display_range(7, 3);
        assert_eq!(range, &["line 7", "line 8", "line 9"]);
    }

    #[test]
    fn disk_spill_preserves_ansi_state() {
        let mut buf = LineBuffer::with_max_history(2, 3);
        buf.push("\x1B[31mred".to_string());
        buf.push("line 2".to_string());
        buf.push("line 3".to_string());
        buf.push("line 4".to_string());
        assert_eq!(buf.disk_line_count, 1);
        let range = buf.display_range(1, 2);
        assert!(range[0].starts_with("\x1B[31m"));
        assert!(range[0].ends_with("line 2"));
    }
}
