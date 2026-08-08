#!/usr/bin/env bash
# Compare public API of sova-core and sova against checked-in baselines.
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
  local features="${2:-}"
  local -a feat_args=()
  if [[ -n "$features" ]]; then
    feat_args=(--features "$features")
  fi
  cargo +nightly public-api -p "$crate" --simplified "${feat_args[@]}" >"$tmpdir/current.txt"
  if ! diff -u "$baseline" "$tmpdir/current.txt"; then
    echo >&2
    echo "Public API of $crate drifted from $baseline" >&2
    echo "If the change is intentional, regenerate with:" >&2
    if [[ -n "$features" ]]; then
      echo "  cargo +nightly public-api -p $crate --features $features --simplified > $baseline" >&2
    else
      echo "  cargo +nightly public-api -p $crate --simplified > $baseline" >&2
    fi
    exit 1
  fi
  echo "public-api baseline OK ($crate)"
}

check_crate sova-core "tls,dev-tls"
check_crate sova "tls,dev-tls,env,store-crypto"
check_crate sova_store unstable-store
check_crate sova-tasks-store unstable-store
