#!/usr/bin/env bash
# Line coverage for Sova libraries (plugins + core/facade). Fail if < 80%.
#
# Requires: cargo install cargo-llvm-cov
#           rustup component add llvm-tools-preview
# Usage:    ./scripts/coverage.sh
# Optional: FAIL_UNDER=80 ./scripts/coverage.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov -V >/dev/null 2>&1; then
  echo "install cargo-llvm-cov: cargo install cargo-llvm-cov --locked" >&2
  exit 1
fi

FAIL_UNDER="${FAIL_UNDER:-80}"
OUT_DIR="${OUT_DIR:-target/llvm-cov}"
mkdir -p "$OUT_DIR"

# Example binaries and cargo-sovax are out of the coverage gate.
EXCLUDES=(
  --exclude cargo-sovax
  --exclude cabinet
  --exclude api_auth
  --exclude api_jwt
  --exclude api_oauth
  --exclude api_preset
  --exclude api_validated
  --exclude auth
  --exclude bench_loaded
  --exclude blog
  --exclude cli
  --exclude crud
  --exclude hello
  --exclude i18n
  --exclude meta_blog
  --exclude quic_udp_echo
  --exclude raw_echo
  --exclude redis_demo
  --exclude rest_api
  --exclude share_demo
  --exclude sse
  --exclude sse_feed
  --exclude static_files
  --exclude storage_demo
  --exclude tasks
  --exclude templates
  --exclude templates_i18n
  --exclude tls_hello
  --exclude udp_echo
  --exclude upload
  --exclude ws_chat
)

# Optional backends that need live Redis / S3 / OAuth IdP are compiled under
# --all-features but skipped at runtime without credentials. Exclude them from
# the line gate so the threshold measures testable library code.
IGNORE_RE='(^|/)(redis(_store)?|opendal_store|messaging|elasticsearch|otel|reqwest_transport)\.rs$|/oauth/|plugins/sova-notifications/src/ws\.rs$'


echo "==> cargo llvm-cov (fail-under-lines=${FAIL_UNDER})"
cargo llvm-cov --workspace \
  --all-features \
  "${EXCLUDES[@]}" \
  --ignore-filename-regex "$IGNORE_RE" \
  --fail-under-lines "$FAIL_UNDER" \
  --lcov \
  --output-path "$OUT_DIR/lcov.info" \
  --summary-only

echo "==> wrote $OUT_DIR/lcov.info"
