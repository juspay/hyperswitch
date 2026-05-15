#!/usr/bin/env bash
# Capture + normalize + strict-replay a connector batch across 3 HS/mitm lanes.
# Usage:
#   cap_replay_all_parallel.sh [connector ...]
# Default connectors exclude nmi and include payout connectors.
set -u

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}
OUT_ROOT=${OUT:-/tmp/mitm_batch_$(date +%s)}
mkdir -p "$OUT_ROOT"

# Keep heavier connectors spread across lanes by ordering defaults intentionally.
if [ "$#" -gt 0 ]; then
  CONNECTORS=("$@")
else
  CONNECTORS=(paypal bluesnap redsys gigadat zift loonio adyenplatform wise)
fi

LANE_0=()
LANE_1=()
LANE_2=()
for idx in "${!CONNECTORS[@]}"; do
  conn=${CONNECTORS[$idx]}
  case $((idx % 3)) in
    0) LANE_0+=("$conn") ;;
    1) LANE_1+=("$conn") ;;
    2) LANE_2+=("$conn") ;;
  esac
done

run_lane() {
  local lane=$1 hs=$2 mport=$3 admin=$4; shift 4
  local connectors=("$@")
  local lane_log="$OUT_ROOT/lane${lane}.log"

  {
    echo "[lane${lane}] start hs=$hs mitm=$mport admin=$admin connectors=${connectors[*]:-<none>}"
    local lane_failed=0
    for conn in "${connectors[@]}"; do
      local conn_out="$OUT_ROOT/$conn"
      mkdir -p "$conn_out"

      echo "[lane${lane}][$conn] capture"
      OUT="$conn_out" bash "$REPO/mitm-proxy/repro/cap_one.sh" "$conn" "$hs" "$mport" "$admin"
      local cap_rc=$?

      local replay_rc=99
      if [ "$cap_rc" -eq 0 ] || [ "${REPLAY_AFTER_FAILED_CAPTURE:-0}" = "1" ]; then
        echo "[lane${lane}][$conn] replay"
        OUT="$conn_out" bash "$REPO/mitm-proxy/repro/replay_one.sh" "$conn" "$hs" "$mport" "$admin"
        replay_rc=$?
      else
        echo "[lane${lane}][$conn] replay skipped because capture failed rc=$cap_rc"
      fi

      printf '%s\tlane%s\tcapture_rc=%s\treplay_rc=%s\n' \
        "$conn" "$lane" "$cap_rc" "$replay_rc" >> "$OUT_ROOT/results.tsv"
      if [ "$cap_rc" -ne 0 ] || [ "$replay_rc" -ne 0 ]; then
        lane_failed=1
      fi
    done
    echo "[lane${lane}] done failed=$lane_failed"
    exit "$lane_failed"
  } > "$lane_log" 2>&1
}

run_lane 0 8080 8888 18097 "${LANE_0[@]}" &
PID0=$!
run_lane 1 8089 8889 18098 "${LANE_1[@]}" &
PID1=$!
run_lane 2 8090 8890 18099 "${LANE_2[@]}" &
PID2=$!

wait "$PID0"; RC0=$?
wait "$PID1"; RC1=$?
wait "$PID2"; RC2=$?

print_connector_summary() {
  local conn=$1
  local conn_out="$OUT_ROOT/$conn"
  local cap_sum replay_sum hitmiss misses
  cap_sum=$(grep -E 'of [0-9]+ (passed|failed)|All specs passed' "$conn_out/recap.$conn.cy.log" 2>/dev/null | tail -1)
  replay_sum=$(grep -E 'of [0-9]+ (passed|failed)|All specs passed' "$conn_out/verify.$conn.cy.log" 2>/dev/null | tail -1)
  hitmiss=$(grep -oE "\[replay\] (HIT|HIT-norm|HIT-replay|HIT-server|SECRET-MISS|MISS|WARN) +\[$conn\]" "$conn_out/verify.$conn.mitm.log" 2>/dev/null | awk '{print $2}' | sort | uniq -c | tr '\n' ' ')
  misses=$(grep -cE "\[replay\] (MISS|SECRET-MISS|WARN) +\[$conn\]" "$conn_out/verify.$conn.mitm.log" 2>/dev/null || true)
  echo "[$conn] capture: ${cap_sum:-<missing>}"
  echo "[$conn] replay : ${replay_sum:-<missing>}"
  echo "[$conn] hit/miss: ${hitmiss:-<none>}"
  echo "[$conn] MISSES  : ${misses:-<unknown>}"
}

echo "OUT_ROOT=$OUT_ROOT"
echo
echo "=== lane rc ==="
echo "lane0 rc=$RC0"
echo "lane1 rc=$RC1"
echo "lane2 rc=$RC2"
echo
echo "=== connector summary ==="
for conn in "${CONNECTORS[@]}"; do
  print_connector_summary "$conn"
  echo
done

if [ "$RC0" -ne 0 ] || [ "$RC1" -ne 0 ] || [ "$RC2" -ne 0 ]; then
  exit 1
fi
