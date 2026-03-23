#!/usr/bin/env bash
# Hit GET /health (curl), refresh lcov from collected .profraw, print path-flow vs LLVM diff.
#
# Prereqs:
#   1) Router built with LLVM coverage and was run with LLVM_PROFILE_FILE pointing under
#      target/coverage-profraw/ (e.g. just run_v2_llvm), OR you already have fresh .profraw.
#   2) Router still running OR you stopped it after exercising /health so .profraw flushed.
#   3) rustup llvm-tools-preview, cargo install grcov, just
#
# Usage (from repo root):
#   ./scripts/run_health_curl_coverage_diff.sh
#   BASE_URL=http://127.0.0.1:3000 ./scripts/run_health_curl_coverage_diff.sh

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
HEALTH_URL="${BASE_URL%/}/health"

echo "==> GET $HEALTH_URL"
if ! out="$(curl -sS -f -w "\nHTTP_CODE=%{http_code}\n" "$HEALTH_URL")"; then
  echo "ERROR: /health request failed. Start Hyperswitch (instrumented if you want new LLVM data)." >&2
  echo "       Try: BASE_URL=http://127.0.0.1:PORT $0" >&2
  exit 1
fi
echo "$out"
echo "==> /health OK"

echo "==> Regenerating lcov + HTML (just coverage_html) …"
just coverage_html

echo "==> Path-flow (health) vs LCOV diff …"
python3 scripts/coverage_feedback_loop.py \
  --chain-artifact scripts/path_flow_health.json \
  --lcov lcov.info \
  --repo-root "$ROOT" \
  --json-only 2>/dev/null | python3 -c "
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
print('---')
"
