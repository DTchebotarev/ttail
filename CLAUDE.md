# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ttail is a Rust CLI tool that buffers terminal output, displaying the last N lines in a scrolling tail view with interactive expand/collapse. Supports two modes: pipe mode (`command | ttail`) and PTY wrapper mode (`ttail command`).

## Build Commands

- **Build:** `cargo build --verbose`
- **Test:** `cargo test --verbose`
- **Run (pipe):** `echo "some\nlines" | cargo run`
- **Run (pty):** `cargo run -- cargo test`

CI runs on Ubuntu via GitHub Actions (`.github/workflows/rust.yml`) on push/PR to main.

## Architecture

Multi-module binary with shared library:

- `src/lib.rs` — `LineBuffer`, `AnsiState`, ANSI color tracking (shared core)
- `src/main.rs` — mode detection (pipe vs pty vs usage), entrypoint
- `src/event.rs` — `Event` enum (`Line`, `Key`, `InputDone`), `Mode` enum
- `src/display.rs` — `clear_lines`, `draw_collapsed`, `draw_expanded`, `write_line`
- `src/pipe.rs` — pipe mode: `steal_stdin`, spawn reader threads, `run_pipe_mode`
- `src/pty.rs` — PTY wrapper mode: `openpty`, `fork+exec`, pty reader, SIGWINCH, key forwarding, `run_pty_mode`
- `src/interactive.rs` — shared event loop (`run_interactive`), non-interactive fallback
- `src/term.rs` — terminal helpers: `terminal_size`, `read_key`, raw mode enable/disable

### Mode detection
- stdin is a pipe → pipe mode (current behavior)
- stdin is a tty + args present → PTY wrapper mode (spawns child in pty)
- stdin is a tty + no args → print usage

### Dependencies
- `libc` — openpty, fork, ioctl, waitpid, tcsetattr (raw mode)
- `signal-hook` — SIGWINCH handling for pty resize
