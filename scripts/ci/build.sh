#!/usr/bin/env bash
set -euo pipefail

cargo build --verbose
cargo test --verbose
