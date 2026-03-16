#!/usr/bin/env bash
set -euo pipefail

output=$(cargo publish 2>&1) && echo "$output" || {
  if echo "$output" | grep -q "already exists"; then
    echo "Already published, skipping"
    exit 0
  fi
  echo "$output"
  exit 1
}
