use std::time::Instant;
use ttail::{LineBuffer, AnsiState, update_ansi_state};

fn bench<F: FnMut()>(name: &str, iterations: u32, mut f: F) {
    // Warmup
    for _ in 0..iterations / 10 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations;
    println!("{name}: {per_iter:?}/iter ({iterations} iterations, {elapsed:?} total)");
}

fn main() {
    let iterations = 10_000;

    bench("push_plain_lines_1k", iterations, || {
        let mut buf = LineBuffer::new(10);
        for i in 0..1000 {
            buf.push(format!("2026-03-15T12:00:{i:02} [INFO] worker: request completed (rid={i})"));
        }
    });

    bench("push_colored_lines_1k", iterations, || {
        let mut buf = LineBuffer::new(10);
        for i in 0..1000 {
            buf.push(format!("\x1B[1;31m2026-03-15T12:00:{i:02}\x1B[0m \x1B[32m[INFO]\x1B[0m worker: request completed"));
        }
    });

    bench("display_lines_plain", iterations, || {
        let mut buf = LineBuffer::new(10);
        for i in 0..100 {
            buf.push(format!("line {i}"));
        }
        std::hint::black_box(buf.display_lines());
    });

    bench("display_lines_with_ansi_prefix", iterations, || {
        let mut buf = LineBuffer::new(10);
        buf.push("\x1B[1;31;44mbold red on blue".to_string());
        for i in 0..100 {
            buf.push(format!("line {i}"));
        }
        std::hint::black_box(buf.display_lines());
    });

    bench("ansi_state_parse_simple", iterations * 10, || {
        let mut state = AnsiState::default();
        update_ansi_state(&mut state, "\x1B[31mhello world");
        std::hint::black_box(&state);
    });

    bench("ansi_state_parse_complex", iterations * 10, || {
        let mut state = AnsiState::default();
        update_ansi_state(&mut state, "\x1B[1;38;5;208m\x1B[48;2;10;20;30mcomplex colors\x1B[0m\x1B[32mgreen");
        std::hint::black_box(&state);
    });

    bench("display_range_middle", iterations, || {
        let mut buf = LineBuffer::new(10);
        for i in 0..1000 {
            buf.push(format!("\x1B[32m[INFO]\x1B[0m line {i}"));
        }
        std::hint::black_box(buf.display_range(500, 20));
    });

    bench("push_and_display_realistic", iterations, || {
        let mut buf = LineBuffer::new(10);
        let lines = [
            "\x1B[90m2026-03-15T12:00:01\x1B[0m \x1B[32m[INFO ]\x1B[0m api: request completed (rid=12345)",
            "\x1B[90m2026-03-15T12:00:02\x1B[0m \x1B[33m[WARN ]\x1B[0m cache: miss for key user:99",
            "\x1B[90m2026-03-15T12:00:03\x1B[0m \x1B[31m[ERROR]\x1B[0m db: connection timed out",
            "\x1B[90m2026-03-15T12:00:04\x1B[0m \x1B[34m[DEBUG]\x1B[0m auth: session validated",
            "\x1B[90m2026-03-15T12:00:05\x1B[0m \x1B[32m[INFO ]\x1B[0m worker: task spawned",
        ];
        for _ in 0..200 {
            for line in &lines {
                buf.push(line.to_string());
                std::hint::black_box(buf.display_lines());
            }
        }
    });
}
