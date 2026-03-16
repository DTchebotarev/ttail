#!/usr/bin/env bash
# Generates 1000 lines of randomish log output to stdout.
# Usage: ./scripts/gen_logs.sh | cargo run

set -euo pipefail

LEVELS=("INFO" "WARN" "ERROR" "DEBUG" "TRACE")
COMPONENTS=("api" "db" "auth" "cache" "scheduler" "worker" "gateway" "metrics")
MESSAGES=(
  "request completed successfully"
  "connection timed out"
  "retrying operation"
  "cache miss for key"
  "user session expired"
  "health check passed"
  "failed to parse payload"
  "rate limit exceeded"
  "query executed"
  "spawning background task"
  "TLS handshake complete"
  "disk usage above threshold"
  "config reloaded"
  "upstream returned 503"
  "record not found"
)

timestamp() {
  if date --version >/dev/null 2>&1; then
    # GNU date
    date -d "+${1} seconds" '+%Y-%m-%dT%H:%M:%S'
  else
    # macOS date
    date -v+"${1}"S '+%Y-%m-%dT%H:%M:%S'
  fi
}

for i in $(seq 1 1000); do
  ts=$(timestamp "$i")
  level=${LEVELS[$((RANDOM % ${#LEVELS[@]}))]}
  comp=${COMPONENTS[$((RANDOM % ${#COMPONENTS[@]}))]}
  msg=${MESSAGES[$((RANDOM % ${#MESSAGES[@]}))]}
  rid=$((RANDOM % 90000 + 10000))
  printf '%s [%-5s] %s: %s (rid=%d)\n' "$ts" "$level" "$comp" "$msg" "$rid"
done
