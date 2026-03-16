use std::io::{self, BufRead, stdout, Write};
use ttail::LineBuffer;

fn clear_lines(num_lines: usize) {
    print!("\x1B[0m");
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
        for l in buf.display_lines() {
            println!("{}", l);
        }
        first = false;
    }
}
