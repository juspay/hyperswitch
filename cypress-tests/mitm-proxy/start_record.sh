#!/usr/bin/env bash
# Start mitmproxy in recording mode.
#
# Usage:
#   ./start.sh                  # default port 8888
#   PROXY_PORT=9090 ./start.sh

set -euo pipefail

PROXY_PORT="${PROXY_PORT:-8888}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Verify required tooling is installed ──────────────────────────────────
for _tool in uv mitmdump; do
  if ! command -v "${_tool}" >/dev/null 2>&1; then
    echo "ERROR: required tool '${_tool}' not found on PATH." >&2
    echo "       Install uv (https://docs.astral.sh/uv/getting-started/installation/) and" >&2
    echo "       mitmproxy (https://docs.mitmproxy.org/stable/overview-installation/)." >&2
    exit 1
  fi
done

# ── Generate mitmproxy CA cert on first run ────────────────────────────────
MITM_CERT_PATH="${HOME}/.mitmproxy/mitmproxy-ca-cert.pem"
if [[ ! -f "${MITM_CERT_PATH}" ]]; then
  echo "==> First run: generating mitmproxy CA cert..."
  timeout 3 mitmdump --listen-port "${PROXY_PORT}" 2>/dev/null || true
fi

if [[ ! -f "${MITM_CERT_PATH}" ]]; then
  echo "ERROR: could not generate mitmproxy CA cert at ${MITM_CERT_PATH}"
  exit 1
fi

# Encode newlines as literal \r\n sequences — this is what Hyperswitch's
# apply_mitm_certificate() expects (it does replace("\\r\\n", "\n") to decode)
MITM_CERT=$(sed 's/$/\\r\\n/' "${MITM_CERT_PATH}" | tr -d '\n')

# ── Print env vars BEFORE blocking on mitmproxy ───────────────────────────
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
echo "==> Starting mitmproxy on :${PROXY_PORT}  (Ctrl+C to stop)"
echo "    Captures  -> ${SCRIPT_DIR}/captures/{connector}/{spec}/{context}/.../NNN.json"
echo "    Admin API -> http://127.0.0.1:8001"
echo ""
echo "Set this in the Cypress terminal so cassettes are organised by spec/test:"
echo "    export CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8001"
echo ""

# ── Run mitmproxy in the foreground so errors are visible ─────────────────
exec uv run --with-requirements "${SCRIPT_DIR}/requirements.txt" \
  mitmdump \
  -s "${SCRIPT_DIR}/mitm_capture.py" \
  --listen-port "${PROXY_PORT}"
