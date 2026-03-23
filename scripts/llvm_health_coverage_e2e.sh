#!/usr/bin/env bash
# Start instrumented router → curl shallow health → graceful stop → grcov (just coverage_html) → path-flow diff + line hits.
#
# Matches env layout of `just run_v2_llvm` (RUSTFLAGS, LLVM_PROFILE_FILE under target/coverage-profraw/).
#
# Prereqs:
#   - Postgres + Redis (and valid config) so the router can boot — same as normal local run.
#   - rustup llvm-tools-preview, cargo install grcov, jq, curl, just
#
# Usage (repo root):
#   ./scripts/llvm_health_coverage_e2e.sh
#   BASE_URL=http://127.0.0.1:8080 HEALTH_PATH=/health SKIP_BUILD=1 ./scripts/llvm_health_coverage_e2e.sh
#   CONFIG_FILE=config/development.toml ./scripts/llvm_health_coverage_e2e.sh
#
# Env:
#   BASE_URL          default http://127.0.0.1:8080
#   HEALTH_PATH       default /v2/health (matches v2-only `just run_v2_llvm`; use /health if you run v1)
#   SKIP_BUILD        if 1, skip cargo build (binary must already match v2 feature set + instrumented)
#   HEALTH_TIMEOUT_S  max seconds to wait for health (default 120)
#   CONFIG_FILE       optional; passed to router as -f (absolute or relative to repo root)
#   ROUTER_LOG        default target/coverage-profraw/router-e2e.log

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
# v2 router registers shallow health under /v2/health (see crates/router/src/routes/app.rs).
HEALTH_PATH="${HEALTH_PATH:-/v2/health}"
HEALTH_URL="${BASE_URL%/}${HEALTH_PATH}"
SKIP_BUILD="${SKIP_BUILD:-0}"
HEALTH_TIMEOUT_S="${HEALTH_TIMEOUT_S:-120}"
ROUTER_LOG="${ROUTER_LOG:-$ROOT/target/coverage-profraw/router-e2e.log}"
CONFIG_FILE="${CONFIG_FILE:-}"

command -v jq >/dev/null 2>&1 || { echo "Need jq (brew install jq)" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "Need curl" >&2; exit 1; }
command -v just >/dev/null 2>&1 || { echo "Need just" >&2; exit 1; }

mkdir -p "$ROOT/target/coverage-profraw"
export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="${ROOT}/target/coverage-profraw/router-%p-%m.profraw"

FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | jq -r '
    [ .packages[] | select(.name == "router") | .features | keys[]
    | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ]
    | join(",")
')"

ROUTER_BIN="$ROOT/target/debug/router"
ROUTER_PID=""

cleanup() {
  if [[ -n "${ROUTER_PID}" ]] && kill -0 "${ROUTER_PID}" 2>/dev/null; then
    echo "==> Sending SIGTERM to router (pid ${ROUTER_PID}) …"
    kill -TERM "${ROUTER_PID}" 2>/dev/null || true
    # Wait for exit so LLVM can flush .profraw
    wait "${ROUTER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [[ "${SKIP_BUILD}" != "1" ]]; then
  echo "==> Building instrumented router (v2 features) …"
  cargo build --package router --no-default-features --features "${FEATURES}"
else
  echo "==> SKIP_BUILD=1 — using existing ${ROUTER_BIN}"
fi

if [[ ! -x "$ROUTER_BIN" ]]; then
  echo "ERROR: missing executable $ROUTER_BIN (build first or unset SKIP_BUILD)" >&2
  exit 1
fi

ROUTER_CMD=( "$ROUTER_BIN" )
if [[ -n "${CONFIG_FILE}" ]]; then
  if [[ "${CONFIG_FILE}" = /* ]]; then
    ROUTER_CMD+=( -f "${CONFIG_FILE}" )
  else
    ROUTER_CMD+=( -f "${ROOT}/${CONFIG_FILE}" )
  fi
fi

echo "==> Starting router (logs: ${ROUTER_LOG}) …"
: >"$ROUTER_LOG"
"${ROUTER_CMD[@]}" >>"$ROUTER_LOG" 2>&1 &
ROUTER_PID=$!

echo "==> Waiting for GET ${HEALTH_URL} (up to ${HEALTH_TIMEOUT_S}s) …"
ok=0
for _ in $(seq 1 "${HEALTH_TIMEOUT_S}"); do
  if curl -sS -f -o /dev/null "$HEALTH_URL" 2>/dev/null; then
    ok=1
    break
  fi
  if ! kill -0 "${ROUTER_PID}" 2>/dev/null; then
    echo "ERROR: router exited before /health was ready. Tail log:" >&2
    tail -n 80 "$ROUTER_LOG" >&2 || true
    exit 1
  fi
  sleep 1
done

if [[ "$ok" != "1" ]]; then
  echo "ERROR: /health not reachable in ${HEALTH_TIMEOUT_S}s" >&2
  tail -n 80 "$ROUTER_LOG" >&2 || true
  exit 1
fi

echo "==> GET ${HEALTH_URL}"
curl -sS -f -w "\nHTTP_CODE=%{http_code}\n" "$HEALTH_URL"
echo "==> /health OK"

cleanup
trap - EXIT
ROUTER_PID=""

echo "==> Regenerating lcov + HTML (just coverage_html) …"
just coverage_html

echo "==> Path-flow (health) vs LCOV diff + per-line hits …"
# stderr: per-line hit table (--print-line-hits); stdout: JSON for summary extractor
python3 scripts/coverage_feedback_loop.py \
  --chain-artifact scripts/path_flow_health.json \
  --lcov lcov.info \
  --repo-root "$ROOT" \
  --print-line-hits \
  --json-only | python3 -c "
import json, sys
o = json.load(sys.stdin)
d = o.get('d', {})
g = (d.get('gaps') or [{}])[0]
print()
print('--- SUMMARY: health() vs LLVM LCOV ---')
print('  leaf:', d.get('leaf'))
print('  body_span:', g.get('body_span'))
print('  status:', g.get('status'))
print('  lcov_probed_lines:', g.get('lcov_probed_lines'))
print('  lcov_hit_lines:', g.get('lcov_hit_lines'))
print('  lines_without_lcov_da:', g.get('lines_without_lcov_da'))
print('  line_coverage_ratio:', g.get('line_coverage_ratio'))
if g.get('note'):
    print('  note:', g.get('note'))
if d.get('error'):
    print('  error:', d.get('error'))
print('---')
print('(Per-line table was printed above on stderr by coverage_feedback_loop.)')
"
