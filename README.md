# ttail

A small, zero-dependency Rust CLI that displays the last N lines of stdin in a rolling window using ANSI escape codes. Think `tail -f`, but for piped output that overwrites itself in place.

ANSI color codes are preserved — even when the line that set the color has scrolled out of the buffer.

## Install

```sh
cargo install ttail
```

## Usage

```sh
# Pipe any command into ttail
my-noisy-command | ttail

# Great for watching logs, builds, test output, etc.
cargo test 2>&1 | ttail
```

## Development

```sh
cargo build --verbose
cargo test --verbose

# Generate sample log data
./scripts/dev/gen_logs.sh | cargo run

# Run performance benchmarks
./scripts/dev/bench.sh
```
