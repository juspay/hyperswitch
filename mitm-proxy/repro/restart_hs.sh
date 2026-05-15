#!/usr/bin/env bash
# Restart HS:8089 + HS:8090 with the COMPLETE pilot config, matching the
# known-good HS:8080 reference:
#   proxy.http_url/https_url  -> per-instance mitm port
#   proxy.mitm_ca_certificate -> Some
#   proxy.mitm_enabled        -> Some(true)
#   trace_header.id_reuse_strategy -> UseIncoming
#   multitenancy tenant base_url    -> per-instance port
# Verifies ALL of these in the startup config dump before declaring OK.
set -u
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}
pause() { timeout "$1" tail -f /dev/null 2>/dev/null || true; }

CERT_PATH="$HOME/.mitmproxy/mitmproxy-ca-cert.pem"
[ -f "$CERT_PATH" ] || { echo "CERT MISSING: $CERT_PATH"; exit 1; }
MITM_CERT=$(sed 's/$/\\r\\n/' "$CERT_PATH" | tr -d '\n')

pid_on_port() {
  ss -ltnp 2>/dev/null | grep -oE ":$1 .*pid=[0-9]+" | grep -oE 'pid=[0-9]+' | head -1 | cut -d= -f2
}

restart_hs() {  # <port> <proxyport> <logfile>
  local port=$1 pp=$2 log=$3
  local oldpid; oldpid=$(pid_on_port "$port")
  echo "### HS:$port (old pid=${oldpid:-none}) -> restart with COMPLETE config"
  [ -n "$oldpid" ] && kill "$oldpid" 2>/dev/null
  local freed=0
  for i in $(seq 1 20); do [ -z "$(pid_on_port "$port")" ] && { freed=1; break; }; pause 1; done
  [ "$freed" = 1 ] || { echo "  port $port still in use — abort"; return 1; }
  echo "  port $port freed"
  cd "$REPO"
  ROUTER__PROXY__HTTPS_URL="http://127.0.0.1:$pp" \
  ROUTER__PROXY__HTTP_URL="http://127.0.0.1:$pp" \
  ROUTER__PROXY__MITM_ENABLED=true \
  ROUTER__PROXY__MITM_CA_CERTIFICATE="$MITM_CERT" \
  ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY=use_incoming \
  ROUTER__SERVER__PORT="$port" \
  ROUTER__MULTITENANCY__TENANTS__PUBLIC__BASE_URL="http://localhost:$port" \
  RUST_MIN_STACK=11534336 \
    nohup target/debug/router > "$log" 2>&1 &
  local up=0
  for i in $(seq 1 60); do curl -sf -m1 "http://localhost:$port/health" >/dev/null 2>&1 && { up=1; break; }; pause 1; done
  [ "$up" = 1 ] || { echo "  HS:$port DID NOT COME UP"; tail -25 "$log"; return 1; }

  # ---- verify ALL config items against the HS:8080 working reference ----
  # HS logs contain ANSI escapes -> grep sees them as binary; strip them first.
  local cfg; cfg=$(grep -m1 -a 'startup_config' "$log" | sed 's/\x1b\[[0-9;]*m//g')
  local ok=1
  check() { # <label> <regex> <expected-substr>
    local got; got=$(echo "$cfg" | grep -oE "$2" | head -1)
    if [[ "$got" == *"$3"* ]]; then echo "    OK  $1: $got"
    else echo "    !!  $1: got '$got' expected to contain '$3'"; ok=0; fi
  }
  echo "  HS:$port healthy (pid=$(pid_on_port "$port")) — verifying config:"
  check "mitm_ca_certificate" 'mitm_ca_certificate: (Some|None)[^,)]*' 'Some'
  check "mitm_enabled"        'mitm_enabled: (Some\(true\)|Some\(false\)|None)' 'Some(true)'
  check "id_reuse_strategy"   'id_reuse_strategy: [A-Za-z]+' 'UseIncoming'
  check "tenant base_url"     'TenantId\("public"\)[^}]*?base_url: "[^"]*"' "localhost:$port"
  check "proxy http_url"      'http_url: Some\("[^"]*"\)' ":$pp"
  [ "$ok" = 1 ] || { echo "  !! HS:$port config INCOMPLETE — abort"; return 1; }
  echo "  HS:$port OK — all 5 config items verified"
}

restart_hs 8089 8889 /tmp/hs2_recap.log || exit 1
restart_hs 8090 8890 /tmp/hs3_recap.log || exit 1
echo "ALL HS RESTARTED OK"
