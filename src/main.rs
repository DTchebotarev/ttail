use std::io::{self, BufRead, stdout, Write};

struct LineBuffer {
    lines: Vec<String>,
    capacity: usize,
}

impl LineBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            lines: Vec::new(),
            capacity,
        }
    }

    fn push(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > self.capacity {
            self.lines.remove(0);
        }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }

    fn lines(&self) -> &[String] {
        &self.lines
    }
}

fn clear_lines(num_lines: usize) {
    for _ in 0..num_lines {
        print!("\x1B[1A");
        print!("\x1B[2K");
    }
    stdout().flush().unwrap();
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock().lines();
    let mut buf = LineBuffer::new(10);
    let mut first = true;

    while let Some(Ok(line)) = input.next() {
        if line.trim().is_empty() {
            break;
        }
        if !first {
            clear_lines(buf.len());
        }
        buf.push(line.trim().to_string());
        for l in buf.lines() {
            println!("{}", l);
        }
        first = false;
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
        assert_eq!(buf.lines(), &[] as &[String]);
    }
}
