#!/usr/bin/env bash
# Start mitmproxy in recording mode + redirect proxy (background).
#
# Usage:
#   ./start_record.sh                  # default port 8888
#   PROXY_PORT=9090 ./start_record.sh

set -euo pipefail

PROXY_PORT="${PROXY_PORT:-8888}"
REDIRECT_PROXY_PORT="${REDIRECT_PROXY_PORT:-9001}"
REDIRECT_PROXY_ADMIN_PORT="${REDIRECT_PROXY_ADMIN_PORT:-9002}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Verify required tooling is installed ──────────────────────────────────
for _tool in uv mitmdump python3; do
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

# ── Start redirect proxy in background ────────────────────────────────────
echo "==> Starting redirect proxy on :${REDIRECT_PROXY_PORT} (admin :${REDIRECT_PROXY_ADMIN_PORT}) ..."
REDIRECT_PROXY_PORT="${REDIRECT_PROXY_PORT}" \
REDIRECT_PROXY_ADMIN_PORT="${REDIRECT_PROXY_ADMIN_PORT}" \
  uv run --with-requirements "${SCRIPT_DIR}/requirements.txt" \
  python3 "${SCRIPT_DIR}/redirect_proxy.py" \
  > /tmp/redirect_proxy.log 2>&1 &
REDIRECT_PROXY_PID=$!

# Give it a moment to bind
sleep 1
if ! kill -0 "${REDIRECT_PROXY_PID}" 2>/dev/null; then
  echo "ERROR: redirect proxy failed to start. Check /tmp/redirect_proxy.log" >&2
  exit 1
fi
echo "    Redirect proxy running (PID=${REDIRECT_PROXY_PID}), log: /tmp/redirect_proxy.log"

# Kill redirect proxy when this script exits (Ctrl+C or error)
trap 'echo ""; echo "==> Stopping redirect proxy (PID=${REDIRECT_PROXY_PID})..."; kill "${REDIRECT_PROXY_PID}" 2>/dev/null || true' EXIT

# ── Print env vars BEFORE blocking on mitmproxy ───────────────────────────
echo ""
echo "Copy these exports into the Hyperswitch terminal, then start the router:"
echo "──────────────────────────────────────────────────────────────────────"
echo "export ROUTER__PROXY__HTTPS_URL=\"http://127.0.0.1:${PROXY_PORT}\""
echo "export ROUTER__PROXY__HTTP_URL=\"http://127.0.0.1:${PROXY_PORT}\""
echo "export ROUTER__PROXY__MITM_CA_CERTIFICATE=\"${MITM_CERT}\""
echo "export ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY=\"use_incoming\""
echo "export ROUTER__MULTITENANCY__TENANTS__PUBLIC__BASE_URL=\"http://localhost:${REDIRECT_PROXY_PORT}\""
echo "export RUST_MIN_STACK=134217728"
echo "cargo run --bin router"
echo "──────────────────────────────────────────────────────────────────────"
echo ""
echo "==> Starting mitmproxy on :${PROXY_PORT}  (Ctrl+C to stop both proxies)"
echo "    Captures  -> ${SCRIPT_DIR}/captures/{connector}/{spec}/{context}/.../NNN.json"
echo "    Admin API -> http://127.0.0.1:8001"
echo ""
echo "Set these in the Cypress terminal:"
echo "    export CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8001"
echo "    export CYPRESS_REDIRECT_PROXY_ADMIN_URL=http://127.0.0.1:${REDIRECT_PROXY_ADMIN_PORT}"
echo "    export CYPRESS_IS_PROXY_ENABLED=true"
echo ""

# ── Run mitmproxy in the foreground so errors are visible ─────────────────
exec uv run --with-requirements "${SCRIPT_DIR}/requirements.txt" \
  mitmdump \
  -s "${SCRIPT_DIR}/mitm_capture.py" \
  --listen-port "${PROXY_PORT}"
