#!/usr/bin/env bash
# Capture + connector-scoped normalize + strict replay, one connector at a time.
# This is the CI-friendly runner: only one HS/mitm/admin lane is used, and
# each connector's replay loads only that connector's capture tree.
#
# Usage:
#   cap_replay_all_one_at_a_time.sh [connector ...]
set -u

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}
OUT_ROOT=${OUT:-/tmp/mitm_batch_one_at_a_time_$(date +%s)}
HS=${HS:-8080}
MPORT=${MPORT:-8888}
ADMIN=${ADMIN:-18097}
mkdir -p "$OUT_ROOT"

if [ "$#" -gt 0 ]; then
  CONNECTORS=("$@")
else
  CONNECTORS=(paypal bluesnap redsys gigadat zift loonio adyenplatform wise)
fi

FAIL=0
: > "$OUT_ROOT/results.tsv"

echo "OUT_ROOT=$OUT_ROOT"
echo "lane hs=$HS mitm=$MPORT admin=$ADMIN"
echo "connectors=${CONNECTORS[*]}"
echo

for conn in "${CONNECTORS[@]}"; do
  conn_out="$OUT_ROOT/$conn"
  mkdir -p "$conn_out"

  echo "=== [$conn] capture ==="
  OUT="$conn_out" bash "$REPO/mitm-proxy/repro/cap_one.sh" "$conn" "$HS" "$MPORT" "$ADMIN"
  cap_rc=$?

  replay_rc=99
  if [ "$cap_rc" -eq 0 ] || [ "${REPLAY_AFTER_FAILED_CAPTURE:-0}" = "1" ]; then
    echo "=== [$conn] replay ==="
    OUT="$conn_out" bash "$REPO/mitm-proxy/repro/replay_one.sh" "$conn" "$HS" "$MPORT" "$ADMIN"
    replay_rc=$?
  else
    echo "=== [$conn] replay skipped because capture failed rc=$cap_rc ==="
  fi

  printf '%s\tcapture_rc=%s\treplay_rc=%s\n' "$conn" "$cap_rc" "$replay_rc" >> "$OUT_ROOT/results.tsv"
  if [ "$cap_rc" -ne 0 ] || [ "$replay_rc" -ne 0 ]; then
    FAIL=1
  fi
  echo

done

echo "=== results.tsv ==="
cat "$OUT_ROOT/results.tsv"
exit "$FAIL"
