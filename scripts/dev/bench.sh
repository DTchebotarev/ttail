#!/usr/bin/env bash
# Run performance benchmarks for ttail.
# Usage: ./scripts/dev/bench.sh
set -euo pipefail

cargo bench --bench line_buffer
