# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ttail is a Rust CLI tool that buffers terminal output, displaying the last N lines in a scrolling tail view with interactive expand/collapse. Two modes: pipe mode (`command | ttail`) and PTY wrapper mode (`ttail command`).

## Build Commands

- **Build:** `cargo build --verbose`
- **Test:** `cargo test --verbose`
- **Run (pipe):** `echo "some\nlines" | cargo run`
- **Run (pty):** `cargo run -- cargo test`

CI runs on Ubuntu via GitHub Actions (`.github/workflows/rust.yml`) on push/PR to main.

## Architecture

Multi-module binary (8 source files) with shared library crate:

- `src/main.rs` — entry point, dispatches between pipe mode and PTY wrapper mode
- `src/lib.rs` — core `LineBuffer` struct with `VecDeque`-based rolling buffer, `AnsiState` tracking for color preservation across buffer eviction
- `src/display.rs` — terminal drawing functions (collapsed view, expanded view, clear/redraw)
- `src/event.rs` — event and key code enums (`Event::Line`, `Event::PtyOutput`, `Event::Key`, `Event::InputDone`), `Mode` enum
- `src/interactive.rs` — main event loop (`run_interactive`) handling events, mode toggling, scrolling, countdown timer; also `run_non_interactive` fallback
- `src/pipe.rs` — pipe mode setup (stdin reader + `/dev/tty` key reader threads)
- `src/pty.rs` — PTY wrapper mode (`openpty`, `fork`+`exec`, pty reader, key forwarding, SIGWINCH handling)
- `src/term.rs` — terminal control (raw mode enable/disable, terminal size query, key parsing)

### Mode detection
- stdin is a pipe → pipe mode
- stdin is a tty + args present → PTY wrapper mode (spawns child in pty)
- stdin is a tty + no args → print usage

### Dependencies
- `libc` — openpty, fork, ioctl, waitpid, tcsetattr (raw mode)
- `signal-hook` — SIGWINCH handling for pty resize
- No other external dependencies
