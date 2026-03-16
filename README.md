# ttail

A small Rust CLI that displays the last N lines of output in a rolling window using ANSI escape codes. Think `tail -f`, but for piped output that overwrites itself in place.

ANSI color codes are preserved — even when the line that set the color has scrolled out of the buffer.

## Features

- **Pipe mode**: `command | ttail` — tails piped output
- **PTY wrapper mode**: `ttail command [args...]` — wraps a command in a pseudo-terminal
- **Interactive expand/collapse**: press Tab to toggle a full scrollable view (vim-style j/k, Page Up/Down, Home/End)
- **Color preservation**: ANSI state tracked across buffer eviction
- **Bounded memory**: in-memory buffer capped at 10k lines, overflow spills to a temp file (auto-cleaned)
- **Auto-exit**: countdown timer after input completes (configurable via `TTAIL_COUNTDOWN_SECS`)

## Install

```sh
cargo install ttail
```

## Usage

```sh
# Pipe any command into ttail
my-noisy-command | ttail

# Wrap a command in a pty
ttail cargo test

# Great for watching logs, builds, test output, etc.
cargo test 2>&1 | ttail
```

## Controls

| Key | Action |
|-----|--------|
| Tab | Toggle expanded scroll view |
| j/k | Scroll up/down (expanded) |
| Page Up/Down | Scroll by page |
| Home/End (g/G) | Jump to start/end |
| q / Ctrl+C | Quit |

## Performance

Benchmarks on the core `LineBuffer` (Apple M-series, single-threaded):

| Operation | Time |
|-----------|------|
| Push 1k plain lines | ~95μs |
| Push 1k colored lines | ~106μs |
| Display 10-line window | ~4.4μs |
| Parse simple SGR sequence | ~12ns |
| Parse complex SGR (bold + palette + RGB) | ~78ns |
| Display range from middle of 1k lines | ~77μs |
| Realistic workload (200 × 5 colored log lines) | ~301μs |

## Development

```sh
cargo build --verbose
cargo test --verbose

# Generate sample log data
./scripts/dev/gen_logs.sh | cargo run

# Run performance benchmarks
cargo bench
```
