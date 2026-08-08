#!/usr/bin/env bash
# Smoke load test against a running Sova server (default hello on :3000).
# Requires: oha (https://github.com/hatoo/oha)
set -euo pipefail

URL="${1:-http://127.0.0.1:3000/}"
DURATION="${DURATION:-10s}"
CONCURRENCY="${CONCURRENCY:-50}"

if ! command -v oha >/dev/null 2>&1; then
  echo "install oha: cargo install oha" >&2
  exit 1
fi

echo "oha -z $DURATION -c $CONCURRENCY $URL"
oha -z "$DURATION" -c "$CONCURRENCY" --no-tui "$URL"
