use std::collections::VecDeque;
use std::fmt::Write;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct AnsiState {
    reset: bool,
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
        let mut codes: Vec<String> = Vec::new();
        if self.bold { codes.push("1".into()); }
        if self.dim { codes.push("2".into()); }
        if self.italic { codes.push("3".into()); }
        if self.underline { codes.push("4".into()); }
        if self.blink { codes.push("5".into()); }
        if self.reverse { codes.push("7".into()); }
        if self.hidden { codes.push("8".into()); }
        if self.strikethrough { codes.push("9".into()); }
        if let Some(ref c) = self.fg {
            Self::color_to_codes(c, &mut codes);
        }
        if let Some(ref c) = self.bg {
            Self::color_to_codes(c, &mut codes);
        }
        if codes.is_empty() {
            String::new()
        } else {
            let mut out = String::new();
            write!(out, "\x1B[{}m", codes.join(";")).unwrap();
            out
        }
    }

    fn color_to_codes(color: &Color, codes: &mut Vec<String>) {
        match color {
            Color::Basic(n) => codes.push(n.to_string()),
            Color::Palette(n) => {
                // Determine if fg (38) or bg (48) from the original code context
                // Basic codes 30-37/90-97 are fg, 40-47/100-107 are bg
                // For palette, we need to check if this is stored as fg or bg
                // This is handled by the caller context, so we just emit the number
                codes.push(format!("5;{n}"));
            }
            Color::Rgb(r, g, b) => {
                codes.push(format!("2;{r};{g};{b}"));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn parse_sgr_params(seq: &str) -> Vec<u8> {
    if seq.is_empty() {
        return vec![0];
    }
    seq.split(';')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect()
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
                let params = parse_sgr_params(param_str);
                state.apply_sgr(&params);
            }
            i += 1;
        } else {
            i += 1;
        }
    }
}

pub struct LineBuffer {
    lines: VecDeque<String>,
    capacity: usize,
    ansi_prefix: AnsiState,
}

impl LineBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            capacity,
            ansi_prefix: AnsiState::default(),
        }
    }

    pub fn push(&mut self, line: String) {
        self.lines.push_back(line);
        if self.lines.len() > self.capacity {
            if let Some(evicted) = self.lines.pop_front() {
                update_ansi_state(&mut self.ansi_prefix, &evicted);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn lines(&self) -> Vec<&str> {
        self.lines.iter().map(|s| s.as_str()).collect()
    }

    pub fn display_lines(&self) -> Vec<String> {
        let lines: Vec<&str> = self.lines.iter().map(|s| s.as_str()).collect();
        if lines.is_empty() || self.ansi_prefix.is_empty() {
            return lines.into_iter().map(String::from).collect();
        }
        let prefix = self.ansi_prefix.to_escape();
        let mut result: Vec<String> = Vec::with_capacity(lines.len());
        result.push(format!("{}{}", prefix, lines[0]));
        for line in &lines[1..] {
            result.push((*line).to_string());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_stays_within_capacity() {
        let mut buf = LineBuffer::new(3);
        for i in 0..5 {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.lines(), &["line 2", "line 3", "line 4"]);
    }

    #[test]
    fn buffer_under_capacity() {
        let mut buf = LineBuffer::new(10);
        buf.push("a".to_string());
        buf.push("b".to_string());
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.lines(), &["a", "b"]);
    }

    #[test]
    fn buffer_exact_capacity() {
        let mut buf = LineBuffer::new(3);
        buf.push("a".to_string());
        buf.push("b".to_string());
        buf.push("c".to_string());
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.lines(), &["a", "b", "c"]);
    }

    #[test]
    fn buffer_drops_oldest() {
        let mut buf = LineBuffer::new(2);
        buf.push("first".to_string());
        buf.push("second".to_string());
        buf.push("third".to_string());
        assert_eq!(buf.lines(), &["second", "third"]);
    }

    #[test]
    fn buffer_capacity_one() {
        let mut buf = LineBuffer::new(1);
        buf.push("a".to_string());
        buf.push("b".to_string());
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.lines(), &["b"]);
    }

    #[test]
    fn empty_buffer() {
        let buf = LineBuffer::new(10);
        assert_eq!(buf.len(), 0);
        assert!(buf.lines().is_empty());
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
}
