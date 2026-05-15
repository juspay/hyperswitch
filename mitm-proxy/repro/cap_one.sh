#!/usr/bin/env bash
# Capture ONE connector's full Cypress suite against a live sandbox.
# Usage: cap_one.sh <connector> <hs_port> <mitm_port> <admin_port>
set -u
CONN=${1:?connector}; HS=${2:?hs_port}; MPORT=${3:?mitm_port}; ADMIN=${4:?admin_port}
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}
CYP=$REPO/cypress-tests
CREDS=${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-$REPO/creds.json}
CAP=$REPO/mitm-proxy/captures
OUT=${OUT:-/tmp/mitm_repro}
mkdir -p "$OUT"
pause() { timeout "$1" tail -f /dev/null 2>/dev/null || true; }
XVFB_SERVER_NUM=${XVFB_SERVER_NUM:-$((ADMIN - 17900))}

suite_for_connector() {
  case "$1" in
    adyenplatform|wise)
      echo 'payout|cypress/e2e/spec/Payout/**/*|1800'
      ;;
    *)
      echo 'payment|cypress/e2e/spec/Payment/**/*|2400'
      ;;
  esac
}

IFS='|' read -r SUITE GLOB CYP_TIMEOUT <<< "$(suite_for_connector "$CONN")"

echo "[$CONN] capture start $(date +%T)  (suite:$SUITE HS:$HS mitm:$MPORT admin:$ADMIN)"

# fresh dir — prior partial captures are backed up at /tmp/partial_backup_*
[ -d "$CAP/$CONN" ] && rm -rf "$CAP/$CONN"

# start mitm capture
pkill -9 -f "listen-port $MPORT" 2>/dev/null; pause 2
CONNECTOR=$CONN CAPTURE_DIR="$CAP" CREDS_FILE="$CREDS" ADMIN_PORT=$ADMIN PYTHONUNBUFFERED=1 \
  nohup mitmdump -s "$REPO/mitm-proxy/mitm_capture.py" --listen-port "$MPORT" \
  > "$OUT/recap.$CONN.mitm.log" 2>&1 &
MPID=$!
ready=0
for i in $(seq 1 25); do
  curl -sf -m1 -X POST "http://127.0.0.1:$ADMIN/test/end" >/dev/null 2>&1 && { ready=1; break; }
  pause 1
done
[ "$ready" = 1 ] || { echo "[$CONN] mitm capture not ready — abort"; cat "$OUT/recap.$CONN.mitm.log"; kill -9 $MPID 2>/dev/null; exit 1; }
echo "[$CONN] mitm capture pid=$MPID ready"

# run cypress capture (suite glob, NO replay mode)
( cd "$CYP" && timeout "$CYP_TIMEOUT" env CYPRESS_CONNECTOR="$CONN" CYPRESS_BASEURL="http://localhost:$HS" \
    CYPRESS_ADMINAPIKEY="${CYPRESS_ADMINAPIKEY:-test_admin}" CYPRESS_CONNECTOR_AUTH_FILE_PATH="$CREDS" \
    CYPRESS_PROXY_ADMIN_URL="http://127.0.0.1:$ADMIN" \
    xvfb-run --auto-servernum --server-num="$XVFB_SERVER_NUM" \
      --server-args='-screen 0 1280x1024x24' \
      npx cypress run --headless --spec "$GLOB" ) > "$OUT/recap.$CONN.cy.log" 2>&1
RC=$?
kill -9 $MPID 2>/dev/null

CASS=$(find "$CAP/$CONN" -name '*.json' 2>/dev/null | wc -l)
SUMMARY=$(grep -E 'of [0-9]+ (passed|failed)|All specs passed' "$OUT/recap.$CONN.cy.log" | tail -1)
echo "[$CONN] capture done $(date +%T)  rc=$RC  CASSETTES=$CASS"
echo "[$CONN] summary: $SUMMARY"
if [ "$CASS" -eq 0 ]; then
  echo "[$CONN] !!! ZERO CASSETTES — mitm not intercepting; check HS:$HS mitm config"
  exit 2
fi

NORM_LOG="$OUT/recap.$CONN.normalize.log"
CREDS_FILE="$CREDS" python3 "$REPO/mitm-proxy/normalize_captures.py" "$CAP" "$CONN" > "$NORM_LOG" 2>&1
NORM_QUAR=$(grep -E 'orphan duplicate cassettes quarantined:' "$NORM_LOG" | awk -F': ' '{print $2}' | tail -1)
NORM_QUAR=${NORM_QUAR:-0}
CASS_POST=$(find "$CAP/$CONN" -name '*.json' 2>/dev/null | wc -l)
echo "[$CONN] normalize done  quarantined=$NORM_QUAR  CASSETTES_POST=$CASS_POST"
CHECK_LOG="$OUT/recap.$CONN.redaction_check.log"
CREDS_FILE="$CREDS" python3 "$REPO/mitm-proxy/check_cassettes_redacted.py" "$CAP/$CONN" > "$CHECK_LOG" 2>&1
CHECK_RC=$?
if [ "$CHECK_RC" -ne 0 ]; then
  echo "[$CONN] redaction check failed; see $CHECK_LOG"
  exit "$CHECK_RC"
fi
if [ "$RC" -ne 0 ]; then
  echo "[$CONN] capture Cypress failed rc=$RC (cassettes preserved and normalized)"
  exit "$RC"
fi
echo "[$CONN] OK"
