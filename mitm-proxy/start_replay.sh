#!/usr/bin/env bash
# Start mitmproxy in replay mode — serves cassettes instead of real connectors.
#
# Usage:
#   ./start_replay.sh                  # default port 8888
#   PROXY_PORT=9090 ./start_replay.sh
#
# Env overrides (also used by CI):
#   CAPTURE_DIR   directory to load cassettes from (default: <script_dir>/captures)
#   ADMIN_PORT    test-lifecycle admin port (default: 8001, read by mitm_replay.py)

set -euo pipefail

PROXY_PORT="${PROXY_PORT:-8888}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CAPTURE_DIR="${CAPTURE_DIR:-${SCRIPT_DIR}/captures}"
export CAPTURE_DIR

if [[ -z "$(find "${CAPTURE_DIR}" -name '*.json' 2>/dev/null)" ]]; then
  echo "ERROR: No cassettes found in ${CAPTURE_DIR}/. Record first (./start.sh) or set CAPTURE_DIR."
  exit 1
fi

MITM_CERT_PATH="${HOME}/.mitmproxy/mitmproxy-ca-cert.pem"
if [[ ! -f "${MITM_CERT_PATH}" ]]; then
  echo "ERROR: mitmproxy CA cert not found at ${MITM_CERT_PATH}. Run mitmdump once to generate it."
  exit 1
fi

# Local-dev hand-holding: only show when stdout is a terminal.  CI runs this
# non-interactively and already sets the router exports itself.
if [[ -t 1 ]]; then
  MITM_CERT=$(sed 's/$/\\r\\n/' "${MITM_CERT_PATH}" | tr -d '\n')

  echo ""
  echo "Copy these exports into the Hyperswitch terminal, then start the router:"
  echo "──────────────────────────────────────────────────────────────────────"
  echo "export ROUTER__PROXY__HTTPS_URL=\"http://127.0.0.1:${PROXY_PORT}\""
  echo "export ROUTER__PROXY__HTTP_URL=\"http://127.0.0.1:${PROXY_PORT}\""
  echo "export ROUTER__PROXY__MITM_ENABLED=\"true\""
  echo "export ROUTER__PROXY__MITM_CA_CERTIFICATE=\"${MITM_CERT}\""
  echo "export ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY=\"use_incoming\""
  echo "cargo run --bin router"
  echo "──────────────────────────────────────────────────────────────────────"
  echo ""
  echo "==> Starting mitmproxy (replay mode) on :${PROXY_PORT}  (Ctrl+C to stop)"
  echo "    HIT  = served from cassette"
  echo "    MISS = cassette not found — request forwarded live"
  echo "    LIVE = request arrived with no x-request-id — forwarded live"
  echo ""
  echo "Optional, for log breadcrumbs in the Cypress terminal:"
  echo "    export CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:${ADMIN_PORT:-8001}"
  echo ""
fi

exec uv run --with-requirements "${SCRIPT_DIR}/requirements.txt" \
  mitmdump \
  -s "${SCRIPT_DIR}/mitm_replay.py" \
  --listen-port "${PROXY_PORT}"
