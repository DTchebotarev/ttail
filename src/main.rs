use std::io::{self, BufRead, stdout, Write};
fn clear_lines(num_lines: usize) {
    for _ in 0..num_lines {
        // Move cursor up one line
        print!("\x1B[1A");
        // Clear the line
        print!("\x1B[2K");
    }
    stdout().flush().unwrap();
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut last10 = Vec::new();
    let mut first = true;

    while let Some(Ok(line)) = lines.next() {
        if line.trim().is_empty() {
            break;
        }
        // Push the new line, and keep only the last 10
        if !first {
            clear_lines(last10.len());
        }
        last10.push(line.trim().to_string());
        if last10.len() > 10 {
            last10.remove(0); // Remove the oldest entry to keep only the last 10 lines
        }
        for i in &last10 {
            println!("{}",i)
        }
        first = false;
    }

}
