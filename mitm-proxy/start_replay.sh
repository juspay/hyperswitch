#!/usr/bin/env bash
# Start mitmproxy in replay mode — serves cassettes instead of real connectors.
#
# Usage:
#   ./start_replay.sh                  # default port 8888
#   PROXY_PORT=9090 ./start_replay.sh

set -euo pipefail

PROXY_PORT="${PROXY_PORT:-8888}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -z "$(find "${SCRIPT_DIR}/captures" -name '*.json' 2>/dev/null)" ]]; then
  echo "ERROR: No cassettes found in captures/. Run ./start.sh and record first."
  exit 1
fi

MITM_CERT_PATH="${HOME}/.mitmproxy/mitmproxy-ca-cert.pem"
if [[ ! -f "${MITM_CERT_PATH}" ]]; then
  echo "ERROR: mitmproxy CA cert not found. Run ./start.sh at least once first."
  exit 1
fi

MITM_CERT=$(sed 's/$/\\r\\n/' "${MITM_CERT_PATH}" | tr -d '\n')

echo ""
echo "Copy these exports into the Hyperswitch terminal, then start the router:"
echo "──────────────────────────────────────────────────────────────────────"
echo "export ROUTER__PROXY__HTTPS_URL=\"http://127.0.0.1:${PROXY_PORT}\""
echo "export ROUTER__PROXY__HTTP_URL=\"http://127.0.0.1:${PROXY_PORT}\""
echo "export ROUTER__PROXY__MITM_CA_CERTIFICATE=\"${MITM_CERT}\""
echo "export ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY=\"use_incoming\""
echo "cargo run --bin router"
echo "──────────────────────────────────────────────────────────────────────"
echo ""
echo "==> Starting mitmproxy (replay mode) on :${PROXY_PORT}  (Ctrl+C to stop)"
echo "    HIT  = served from cassette"
echo "    MISS = no cassette found, forwarded live"
echo "    WARN = request arrived before /test/start was called"
echo ""

exec mitmdump \
  -s "${SCRIPT_DIR}/mitm_replay.py" \
  --listen-port "${PROXY_PORT}"
