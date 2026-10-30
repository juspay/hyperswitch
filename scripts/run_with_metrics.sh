#!/usr/bin/env bash
# Run Cypress (or another command) alongside the incremental metric watcher.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
runner_pid="" watcher_pid=""
stopped_pids=()

# shellcheck disable=SC2329 # Called indirectly by the EXIT cleanup trap.
freeze_tree() {
  local pid="$1" child
  # Freeze parents before discovering children so they cannot start more jobs.
  # GNU Parallel jobs have separate process groups, so a group kill alone is
  # insufficient. Traverse descendants, including npm, browsers, and Xvfb.
  kill -STOP "$pid" 2>/dev/null || return 0
  stopped_pids+=("$pid")
  while read -r child; do
    [[ -z "$child" ]] || freeze_tree "$child"
  done < <(pgrep -P "$pid" || true)
}

# shellcheck disable=SC2329 # Invoked by trap.
cleanup() {
  local status=$?
  trap - EXIT INT TERM
  [[ -z "$runner_pid" ]] || freeze_tree "$runner_pid"
  [[ -z "$watcher_pid" ]] || freeze_tree "$watcher_pid"
  if (( ${#stopped_pids[@]} )); then
    kill -TERM "${stopped_pids[@]}" 2>/dev/null || true
    kill -CONT "${stopped_pids[@]}" 2>/dev/null || true
    sleep 1
    kill -KILL "${stopped_pids[@]}" 2>/dev/null || true
  fi
  [[ -z "$runner_pid" ]] || wait "$runner_pid" 2>/dev/null || true
  [[ -z "$watcher_pid" ]] || wait "$watcher_pid" 2>/dev/null || true
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if (( $# == 0 )); then
  echo "Usage: $0 COMMAND [ARGUMENTS...]" >&2
  exit 1
fi
# Check process discovery before launching anything; cleanup needs it to find
# workers in GNU Parallels separate process groups.
pgrep -P "$$" >/dev/null || {
  status=$?
  if (( status > 1 )); then
    echo "ERROR: Cannot inspect child processes for metric-triggered cleanup" >&2
    exit 1
  fi
}
"$@" &
runner_pid=$!
bash "$script_dir/check_metrics.sh" --watch &
watcher_pid=$!

while kill -0 "$runner_pid" 2>/dev/null && kill -0 "$watcher_pid" 2>/dev/null; do
  sleep 0.2
done
if ! kill -0 "$watcher_pid" 2>/dev/null; then
  wait "$watcher_pid" || true
  watcher_pid=""
  echo "::error::Metric watcher stopped; aborting the Cypress run."
  exit 1
fi

status=0
wait "$runner_pid" || status=$?
runner_pid=""
exit "$status"
