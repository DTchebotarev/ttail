## [0.3.2] - 2026-03-16

### Added
- `-h`/`--help` flag support (exits 0; no-args usage exits 1)
- cargo-dist configuration for automated GitHub Releases
- Homebrew tap (`brew install DTchebotarev/tap/ttail`)
- Shell installer for GitHub Releases

### Changed
- Extracted usage text into `print_usage()` to avoid duplication
- Updated README with Homebrew and shell installer instructions

## [0.3.1] - 2026-03-16

### Changed
- Eliminate heap allocations in ANSI parsing hot path (stack array for `parse_sgr_params`, direct string writes in `to_escape`)
- Remove intermediate `Vec<&str>` in display methods, batch `clear_lines` into single write
- Deduplicate Line/PtyOutput draw logic and unify countdown/collapsed draw functions
- 23–61% faster across all benchmarks

### Added
- Cap VecDeque at 10k lines with automatic disk spill for overflow (no data loss)
- Updated README with features, controls, and performance benchmarks

## [0.3.0] - 2026-03-16

### Added
- Interactive expand/collapse toggle (Tab)
- Expanded mode navigation: ↑/↓/j/k, PgUp/PgDn, Home/End, g/G
- PTY wrapper mode: `ttail command [args...]` spawns child in a pseudo-terminal
- Countdown auto-exit timer after input completes (configurable via `TTAIL_COUNTDOWN_SECS`)
- Non-interactive fallback when no tty is available (CI, piped output)
- Status line showing line count, scroll position, and done state

### Changed
- Dropped `crossterm` dependency in favor of direct raw mode / key parsing via `libc`
- `LineBuffer` retains all lines; windowed display via `display_range()`

## [0.2.0] - 2026-03-15

### Added
- ANSI color preservation across buffer eviction (basic, 256-color, and RGB)
- `LineBuffer` public API with `AnsiState` tracking
- Performance benchmark suite (`cargo bench`)
- Colored log test script for visual ANSI verification
- Unit tests for ANSI state tracking (17 total)
- `/version-bump` skill for semver version management

### Changed
- Extracted core logic into `src/lib.rs` for testability
- Switched internal buffer from `Vec` to `VecDeque` for O(1) eviction
- Reset ANSI state before clearing lines to prevent background color bleed
- Updated README with install/usage/development instructions

### Fixed
- Background colors filling cleared lines via `\x1B[2K]`
- Extended color output missing `38;`/`48;` prefix for palette and RGB
- Stale `Cargo.lock` causing publish failures

## [0.1.1] - 2026-03-15

### Changed
- Bumped Rust edition from 2021 to 2024 (MSRV 1.85)
- Switched license field to SPDX identifier
- Added crates.io metadata (keywords, categories, homepage, rust-version)

### Added
- GitHub Actions workflow for automatic crates.io publishing on version bump
- Log generation test script (`scripts/gen_logs.sh`)

## [0.1.0] - 2024-11-18

### Added

Initial commit.
