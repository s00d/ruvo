#!/usr/bin/env bash
# Compare public API of ruvo-core and ruvo against checked-in baselines.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo-public-api >/dev/null 2>&1 && ! cargo public-api -V >/dev/null 2>&1; then
  echo "install cargo-public-api: cargo install cargo-public-api --locked" >&2
  exit 1
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

check_crate() {
  local crate="$1"
  local baseline="api/${crate}.txt"
  cargo +nightly public-api -p "$crate" --simplified >"$tmpdir/current.txt"
  if ! diff -u "$baseline" "$tmpdir/current.txt"; then
    echo >&2
    echo "Public API of $crate drifted from $baseline" >&2
    echo "If the change is intentional, regenerate with:" >&2
    echo "  cargo +nightly public-api -p $crate --simplified > $baseline" >&2
    exit 1
  fi
  echo "public-api baseline OK ($crate)"
}

check_crate ruvo-core
check_crate ruvo
