#!/usr/bin/env bash
# Compares local Cargo.toml version against what's published on crates.io.
# Outputs to GITHUB_OUTPUT if available, otherwise prints to stdout.
set -euo pipefail

CRATE_NAME="ttail"

LOCAL_VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
PUBLISHED_VERSION=$(curl -sS -A "ttail-ci (https://github.com/DTchebotarev/ttail)" "https://crates.io/api/v1/crates/${CRATE_NAME}" | jq -r '.crate.max_version // ""')

# Default to not publishing if we can't determine published version
CHANGED="false"
if [ -n "$PUBLISHED_VERSION" ] && [ "$LOCAL_VERSION" != "$PUBLISHED_VERSION" ]; then
  CHANGED="true"
fi

echo "local=$LOCAL_VERSION"
echo "published=$PUBLISHED_VERSION"
echo "changed=$CHANGED"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "local=$LOCAL_VERSION" >> "$GITHUB_OUTPUT"
  echo "published=$PUBLISHED_VERSION" >> "$GITHUB_OUTPUT"
  echo "changed=$CHANGED" >> "$GITHUB_OUTPUT"
fi
