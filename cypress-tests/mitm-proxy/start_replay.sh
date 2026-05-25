#!/usr/bin/env bash
# Start mitmproxy in replay mode — serves cassettes instead of real connectors.
#
# Usage:
#   ./start_replay.sh                  # permissive: MISS forwards to live connector
#   ./start_replay.sh --strict         # strict: MISS returns 599, never live
#   PROXY_PORT=9090 ./start_replay.sh
#
# Env overrides (also used by CI):
#   CAPTURE_DIR    directory to load cassettes from (default: <script_dir>/captures)
#   ADMIN_PORT     test-lifecycle admin port (default: 8001, read by mitm_replay.py)
#   REPLAY_STRICT  set to 1 to enable strict mode (same as --strict flag)

set -euo pipefail

STRICT_FLAG=0
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT_FLAG=1 ;;
    *) echo "WARN: unknown argument '${arg}' ignored" >&2 ;;
  esac
done

if [[ "${STRICT_FLAG}" == "1" ]]; then
  export REPLAY_STRICT=1
fi
STRICT_MODE="${REPLAY_STRICT:-0}"

PROXY_PORT="${PROXY_PORT:-8888}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CAPTURE_DIR="${CAPTURE_DIR:-${SCRIPT_DIR}/captures}"
export CAPTURE_DIR

# ── Verify required tooling is installed ──────────────────────────────────
for _tool in uv mitmdump; do
  if ! command -v "${_tool}" >/dev/null 2>&1; then
    echo "ERROR: required tool '${_tool}' not found on PATH." >&2
    echo "       Install uv (https://docs.astral.sh/uv/getting-started/installation/) and" >&2
    echo "       mitmproxy (https://docs.mitmproxy.org/stable/overview-installation/)." >&2
    exit 1
  fi
done

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
  if [[ "${STRICT_MODE}" == "1" ]]; then
    MODE_DESC="STRICT — MISS / no-rid blocked with 599 (connector never called)"
  else
    MODE_DESC="permissive — MISS / no-rid forwarded to live connector"
  fi

  echo "==> Starting mitmproxy (replay mode) on :${PROXY_PORT}  (Ctrl+C to stop)"
  echo "    Mode : ${MODE_DESC}"
  echo "    HIT   = served from cassette"
  echo "    MISS  = cassette not found"
  echo "    LIVE  = request arrived with no x-request-id"
  echo "    BLOCK = strict-mode synthetic 599 (only in --strict)"
  echo ""
  echo "Optional, for log breadcrumbs in the Cypress terminal:"
  echo "    export CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:${ADMIN_PORT:-8001}"
  echo ""
fi

# connection_strategy=lazy: never open an upstream connection on CONNECT.
#   With the default (eager), mitmproxy dials the real connector host the
#   instant the tunnel opens — to clone its TLS cert — before our replay
#   hook runs. In CI that host resolves but egress is blocked, so the dial
#   hangs until Hyperswitch's 30s client timeout fires → 502, cassette never
#   served. lazy defers the dial so HIT/strict-MISS are served immediately;
#   a permissive live-MISS still connects just-in-time.
# upstream_cert=false: generate the client-facing cert from the SNI instead
#   of fetching the real one, so replay never touches the network for certs.
exec uv run --with-requirements "${SCRIPT_DIR}/requirements.txt" \
  mitmdump \
  -s "${SCRIPT_DIR}/mitm_replay.py" \
  --listen-port "${PROXY_PORT}" \
  --set connection_strategy=lazy \
  --set upstream_cert=false
