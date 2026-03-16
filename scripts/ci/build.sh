#!/usr/bin/env bash
set -euo pipefail

cargo build --locked --verbose
cargo test --locked --verbose
