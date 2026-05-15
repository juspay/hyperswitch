#!/usr/bin/env bash
# Strict replay-verify ONE connector's full Cypress suite.
# Usage: replay_one.sh <connector> <hs_port> <mitm_port> <admin_port>
set -u
CONN=${1:?connector}; HS=${2:?hs_port}; MPORT=${3:?mitm_port}; ADMIN=${4:?admin_port}
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}
CYP=$REPO/cypress-tests
CREDS=${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-$REPO/creds.json}
OUT=${OUT:-/tmp/mitm_repro}
mkdir -p "$OUT"
pause() { timeout "$1" tail -f /dev/null 2>/dev/null || true; }
XVFB_SERVER_NUM=${XVFB_SERVER_NUM:-$((ADMIN - 17900))}

suite_for_connector() {
  case "$1" in
    adyenplatform|wise)
      echo 'payout|cypress/e2e/spec/Payout/**/*|900'
      ;;
    *)
      echo 'payment|cypress/e2e/spec/Payment/**/*|1200'
      ;;
  esac
}

IFS='|' read -r SUITE GLOB CYP_TIMEOUT <<< "$(suite_for_connector "$CONN")"

echo "[$CONN] replay-verify start $(date +%T)  (suite:$SUITE HS:$HS mitm:$MPORT admin:$ADMIN)"

# start strict mitm replay for this connector's capture tree only
pkill -9 -f "listen-port $MPORT" 2>/dev/null; pause 2
CAPTURE_DIR="$REPO/mitm-proxy/captures/$CONN" CREDS_FILE="$CREDS" ADMIN_PORT=$ADMIN PYTHONUNBUFFERED=1 \
  nohup mitmdump -s "$REPO/mitm-proxy/repro/mitm_replay_strict.py" --listen-port "$MPORT" \
  > "$OUT/verify.$CONN.mitm.log" 2>&1 &
MPID=$!
ready=0
for i in $(seq 1 30); do
  curl -sf -m1 -X POST "http://127.0.0.1:$ADMIN/test/end" >/dev/null 2>&1 && { ready=1; break; }
  pause 1
done
[ "$ready" = 1 ] || { echo "[$CONN] mitm replay not ready — abort"; cat "$OUT/verify.$CONN.mitm.log"; kill -9 $MPID 2>/dev/null; exit 1; }
echo "[$CONN] mitm replay pid=$MPID ready"

( cd "$CYP" && timeout "$CYP_TIMEOUT" env CYPRESS_CONNECTOR="$CONN" CYPRESS_BASEURL="http://localhost:$HS" \
    CYPRESS_ADMINAPIKEY="${CYPRESS_ADMINAPIKEY:-test_admin}" CYPRESS_CONNECTOR_AUTH_FILE_PATH="$CREDS" \
    CYPRESS_PROXY_ADMIN_URL="http://127.0.0.1:$ADMIN" CYPRESS_PROXY_MODE=replay \
    xvfb-run --auto-servernum --server-num="$XVFB_SERVER_NUM" \
      --server-args='-screen 0 1280x1024x24' \
      npx cypress run --headless --spec "$GLOB" ) > "$OUT/verify.$CONN.cy.log" 2>&1
RC=$?
kill -9 $MPID 2>/dev/null

SUMMARY=$(grep -E 'of [0-9]+ (passed|failed)|All specs passed' "$OUT/verify.$CONN.cy.log" | tail -1)
HITMISS=$(grep -oE "\[replay\] (HIT|HIT-norm|HIT-replay|HIT-server|SECRET-MISS|MISS|WARN) +\[$CONN\]" "$OUT/verify.$CONN.mitm.log" | awk '{print $2}' | sort | uniq -c | tr '\n' ' ')
MISSES=$(grep -cE "\[replay\] (MISS|SECRET-MISS|WARN) +\[$CONN\]" "$OUT/verify.$CONN.mitm.log")
echo "[$CONN] replay done $(date +%T) rc=$RC"
echo "[$CONN] summary : $SUMMARY"
echo "[$CONN] hit/miss: $HITMISS"
echo "[$CONN] MISSES  : $MISSES"
if [ "$MISSES" -gt 0 ]; then
  echo "[$CONN] --- sample replay failures ---"
  grep -E "\[replay\] (MISS|SECRET-MISS|WARN) +\[$CONN\]" "$OUT/verify.$CONN.mitm.log" | head -10
  exit 2
fi
if [ "$RC" -ne 0 ]; then
  echo "[$CONN] replay Cypress failed rc=$RC"
  if [ "${IGNORE_CYPRESS_FAILURES:-0}" != "1" ]; then
    exit "$RC"
  fi
  echo "[$CONN] IGNORE_CYPRESS_FAILURES=1 — keeping job green because connector MISS count is zero"
fi
