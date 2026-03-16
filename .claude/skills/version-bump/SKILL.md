---
name: version-bump
description: Bump the project version (major/minor/patch) across all relevant files
disable-model-invocation: true
argument-hint: "[major|minor|patch]"
---

Bump the project version. If an argument is provided ($ARGUMENTS), use it directly. Otherwise, analyze recent changes and suggest major/minor/patch with reasoning, then ask the user which to apply.

## Versioning guidelines

Use semver (https://semver.org):
- **patch**: Bug fixes, documentation, CI changes, refactoring with no API changes
- **minor**: New features, new public API surface, backwards-compatible changes
- **major**: Breaking changes to public API or behavior

## Steps

1. Read `Cargo.toml` to get the current version
2. Read `CHANGELOG.md` to understand what's changed since the last release
3. Check recent git history since the last version tag: !`git log --oneline $(git describe --tags --abbrev=0 2>/dev/null || echo HEAD~20)..HEAD 2>/dev/null || git log --oneline -20`
4. If no argument was provided, suggest all three options with the resulting version number and reasoning for each, then ask the user to pick one
5. Compute the new version number
6. Update ALL of these files:
   - `Cargo.toml` — the `version` field
   - `Cargo.lock` — run `cargo generate-lockfile` to sync it
   - `CHANGELOG.md` — add a new section header with the new version and today's date, moving any unreleased notes under it
7. Verify with `cargo build --locked` that the lockfile is in sync
8. Show the user a summary of what was changed and the new version number
