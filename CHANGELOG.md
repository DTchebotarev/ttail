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
