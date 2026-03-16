# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ttail is a Rust CLI tool that buffers terminal output, displaying the last N lines of stdin in a scrolling fashion using ANSI escape codes. No external dependencies — pure Rust std lib.

## Build Commands

- **Build:** `cargo build --verbose`
- **Test:** `cargo test --verbose`
- **Run:** `echo "some\nlines" | cargo run`

CI runs on Ubuntu via GitHub Actions (`.github/workflows/rust.yml`) on push/PR to main.

## Architecture

Single-file program in `src/main.rs` with two functions:
- `clear_lines(num_lines)` — uses ANSI escape sequences (`\x1B[1A`, `\x1B[2K`) to clear and redraw terminal lines
- `main()` — reads stdin line-by-line, maintains a rolling buffer of the last 10 lines, clears previous display and reprints on each new line, stops on empty input
