#!/usr/bin/env bash
# Record a fred XDEL on an isolated Redis, then replay its typed result after
# that Redis is stopped. Requires redis-server, redis-cli, python3, and Cargo.
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')
tmp=$(mktemp -d /tmp/fred-xdel-smoke.XXXXXX)
ready="$tmp/ready"
proceed="$tmp/proceed"
cleanup() {
  if [[ -n "${replay_pid:-}" ]]; then
    kill "$replay_pid" 2>/dev/null || true
  fi
  if [[ -n "${redis_pid:-}" ]]; then
    kill "$redis_pid" 2>/dev/null || true
    wait "$redis_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
redis-server --bind 127.0.0.1 --port "$port" --save '' --appendonly no --daemonize no --dir "$tmp" > "$tmp/redis.log" 2>&1 &
redis_pid=$!
for _ in $(seq 1 100); do
  kill -0 "$redis_pid" 2>/dev/null || { echo 'disposable Redis exited before readiness' >&2; exit 1; }
  redis-cli -h 127.0.0.1 -p "$port" ping >/dev/null 2>&1 && break
  sleep 0.05
done
redis-cli -h 127.0.0.1 -p "$port" ping >/dev/null
artifact="$tmp/artifact"
key="fred_xdel_smoke_$RANDOM"
common=(cargo test -p redis_interface --no-default-features --features fred,deja --lib test::deja_fred_xdel_replay_smoke -- --ignored --exact --nocapture)
(
  cd "$repo"
  DEJA_FRED_XDEL_PHASE=record \
  DEJA_FRED_XDEL_ARTIFACT="$artifact" \
  DEJA_FRED_XDEL_KEY="$key" \
  DEJA_FRED_XDEL_REDIS_PORT="$port" \
    "${common[@]}"
)
(
  cd "$repo"
  DEJA_FRED_XDEL_PHASE=replay \
  DEJA_FRED_XDEL_ARTIFACT="$artifact" \
  DEJA_FRED_XDEL_KEY="$key" \
  DEJA_FRED_XDEL_REDIS_PORT="$port" \
  DEJA_FRED_XDEL_READY="$ready" \
  DEJA_FRED_XDEL_PROCEED="$proceed" \
    "${common[@]}"
) &
replay_pid=$!
for _ in $(seq 1 300); do
  [[ -f "$ready" ]] && break
  kill -0 "$replay_pid" 2>/dev/null || { wait "$replay_pid"; exit 1; }
  sleep 0.1
done
[[ -f "$ready" ]] || { echo 'replay did not reach Redis-connected barrier' >&2; exit 1; }
# The connected client must now get XDEL from the recording, not Redis.
server_pid=$(redis-cli --raw -h 127.0.0.1 -p "$port" info server | python3 -c 'import sys; print(next((line.split(":", 1)[1].strip() for line in sys.stdin if line.startswith("process_id:")), ""))')
[[ "$server_pid" == "$redis_pid" ]] || { echo 'Redis port does not belong to disposable instance' >&2; exit 1; }
redis-cli -h 127.0.0.1 -p "$port" shutdown nosave
wait "$redis_pid"
redis_pid=
: > "$proceed"
wait "$replay_pid"
replay_pid=
printf 'record and replay passed; replay returned XDEL count 1 with Redis stopped\n'
