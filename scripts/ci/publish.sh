#!/usr/bin/env bash
set -euo pipefail

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

# Check if this version is already published
if cargo search ttail --limit 1 | grep -q "^ttail = \"$VERSION\""; then
  echo "ttail@$VERSION already published to crates.io, skipping"
  exit 0
fi

cargo publish
