#!/usr/bin/env bash
# LLVM source-based coverage for the Hyperswitch router (POC / local use).
#
# Prereqs:
#   rustup component add llvm-tools-preview
#   cargo install grcov
#
# Usage:
#   1) Build instrumented binary:
#        ./scripts/llvm_coverage_router.sh build
#   2) Run router (separate terminal), with profile output:
#        export LLVM_PROFILE_FILE="$PWD/target/coverage-profraw/router-%p-%m.profraw"
#        ./target/debug/router
#      Exercise the API (Cypress, curl), then stop router with Ctrl+C.
#   3) Generate HTML + lcov:
#        ./scripts/llvm_coverage_router.sh report
#
# Or run a quick self-check on a small crate (no DB):
#        ./scripts/llvm_coverage_router.sh demo

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROF_DIR="${PROF_DIR:-$ROOT/target/coverage-profraw}"
HTML_OUT="${HTML_OUT:-$ROOT/target/coverage-html}"
LCOV_OUT="${LCOV_OUT:-$ROOT/lcov.info}"
GRCOV="${GRCOV:-grcov}"

ensure_tools() {
  command -v "$GRCOV" >/dev/null 2>&1 || {
    echo "Install grcov: cargo install grcov" >&2
    exit 1
  }
  rustup component list --installed | grep -q 'llvm-tools' || {
    echo "Install LLVM tools: rustup component add llvm-tools-preview" >&2
    exit 1
  }
}

cmd_build() {
  export RUSTFLAGS="${RUSTFLAGS:--Cinstrument-coverage}"
  echo "RUSTFLAGS=$RUSTFLAGS"
  cargo build --bin router --package router
  echo "Built: $ROOT/target/debug/router"
}

cmd_report() {
  ensure_tools
  mkdir -p "$HTML_OUT"
  # Merge profraw from default dir + repo root (build scripts may drop some at root)
  "$GRCOV" "$ROOT" \
    --source-dir "$ROOT" \
    --binary-path "$ROOT/target/debug" \
    --output-type html \
    --output-path "$HTML_OUT" \
    --keep-only "crates/*" \
    ${GRCOV_EXTRA_ARGS:-}

  "$GRCOV" "$ROOT" \
    --source-dir "$ROOT" \
    --binary-path "$ROOT/target/debug" \
    --output-type lcov \
    --output-path "$LCOV_OUT" \
    --keep-only "crates/*" \
    ${GRCOV_EXTRA_ARGS:-}

  echo "HTML: $HTML_OUT/index.html"
  echo "LCOV: $LCOV_OUT"
}

# Fast sanity check: instrument router_env tests + grcov (no server).
cmd_demo() {
  ensure_tools
  local demo_prof="$ROOT/target/coverage-poc-demo"
  mkdir -p "$demo_prof"
  export RUSTFLAGS="-Cinstrument-coverage"
  export LLVM_PROFILE_FILE="$demo_prof/poc-%p-%m.profraw"
  # Pin target dir to workspace so grcov binary-path matches
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
  cargo test -p router_env --lib -- --test-threads=1
  local out="$ROOT/target/coverage-poc-html"
  mkdir -p "$out"
  "$GRCOV" "$demo_prof" \
    --source-dir "$ROOT" \
    --binary-path "$CARGO_TARGET_DIR/debug/deps" \
    --output-type html \
    --output-path "$out" \
    --keep-only "crates/router_env/*"
  echo "Demo HTML: $out/index.html"
}

case "${1:-}" in
  build)   cmd_build ;;
  report)  cmd_report ;;
  demo)    cmd_demo ;;
  *)
    echo "Usage: $0 {build|report|demo}" >&2
    echo "" >&2
    echo "  build  - cargo build router with -Cinstrument-coverage" >&2
    echo "  report - grcov -> target/coverage-html + lcov.info (run after collecting .profraw)" >&2
    echo "  demo   - quick test using router_env + HTML under target/coverage-poc-html" >&2
    exit 1
    ;;
esac
