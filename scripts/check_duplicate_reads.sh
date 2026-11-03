#!/usr/bin/env bash
set -euo pipefail

server_log=${1:?Usage: check_duplicate_reads.sh SERVER_LOG_PATH DUPLICATE_READ_LOG_PATH}
duplicate_log=${2:?Usage: check_duplicate_reads.sh SERVER_LOG_PATH DUPLICATE_READ_LOG_PATH}

if [[ ! -f "$server_log" ]]; then
  echo "::error::Router server log not found: $server_log"
  exit 1
fi

grep_status=0
grep -F 'Duplicate database reads in session' "$server_log" > "$duplicate_log" || grep_status=$?
if (( grep_status > 1 )); then
  echo "::error::Could not scan router server log: $server_log"
  exit "$grep_status"
fi

if [[ -s "$duplicate_log" ]]; then
  echo "::error::Duplicate database reads detected. See the duplicate-read log artifact for details."
  exit 1
fi

echo "No duplicate database reads detected."
