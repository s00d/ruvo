#!/usr/bin/env bash
# SCALE: hello vs loaded across tokio worker counts.
# Requires: oha, curl. Writes markdown table to stdout; appends nothing by itself.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DURATION="${DURATION:-5s}"
CONCURRENCY="${CONCURRENCY:-50}"
WORKERS="${WORKERS:-1 2 4 8}"
PROFILE="${1:-both}" # hello | loaded | both
# loaded post-shard: expect Req/s to rise with workers (no ~5k plateau at 4+).
# Compare output against bench/BASELINE.md post-shard table.

if ! command -v oha >/dev/null 2>&1; then
  echo "install oha: cargo install oha" >&2
  exit 1
fi

run_profile() {
  local name="$1"
  local port="$2"
  local example="$3"
  local features="$4"
  local cookie_hdr="${5:-}"

  echo "## $name" >&2
  for w in $WORKERS; do
    echo "building/running $name workers=$w ..." >&2
    TOKIO_WORKER_THREADS="$w" cargo run -q -p ruvo --example "$example" --features "$features" >/tmp/ruvo-scale-$name-$w.log 2>&1 &
    local pid=$!
    cleanup() { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
    trap cleanup EXIT

    for _ in $(seq 1 60); do
      if curl -sf "http://127.0.0.1:$port/" >/dev/null 2>&1; then
        break
      fi
      sleep 0.25
    done

    local extra=()
    if [[ -n "$cookie_hdr" ]]; then
      # Establish session once, reuse cookie jar for load.
      local jar
      jar="$(mktemp)"
      curl -sf -c "$jar" -b "$jar" "http://127.0.0.1:$port/" >/dev/null
      local sid
      sid="$(awk '/ruvo_sid/ {print $7}' "$jar" | tail -1)"
      rm -f "$jar"
      extra=(-H "Cookie: ruvo_sid=${sid}")
    fi

    local out
    out="$(oha -z "$DURATION" -c "$CONCURRENCY" --no-tui "${extra[@]}" "http://127.0.0.1:$port/" 2>&1)"
    local rps p50 p99
    rps="$(echo "$out" | awk '/Requests\/sec/ {print $2; exit}')"
    p50="$(echo "$out" | awk '/50.00%/ {print $3; exit}')"
    p99="$(echo "$out" | awk '/99.00%/ {print $3; exit}')"
    echo "| $name | $w | $rps | ${p50} ms | ${p99} ms |"

    cleanup
    trap - EXIT
    sleep 0.5
  done
}

echo "| Profile | Workers | Req/s | p50 | p99 |"
echo "|---------|---------|-------|-----|-----|"

cd "$ROOT"
if [[ "$PROFILE" == "hello" || "$PROFILE" == "both" ]]; then
  run_profile hello 3000 hello "static-files,cors,cookies"
fi
if [[ "$PROFILE" == "loaded" || "$PROFILE" == "both" ]]; then
  run_profile loaded 3001 bench_loaded "cors,cookies,session,rate-limit" cookie
fi
