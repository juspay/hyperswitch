#!/usr/bin/env bash
# Fail if any recorded counter or gauge exceeds its configured maximum.
# Config: {"DUPLICATE_DB_READ_COUNT": 0, "QUEUE_DEPTH": 100}
# Names match OTLP instrument names exactly. Absent metrics are allowed.
set -euo pipefail

metrics_script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

parse_metrics() {
  jq -nr --slurpfile limits "$1" --arg phase "${2:-final}" '
    def numeric:
      tonumber | if isfinite and (isnan | not) then . else error("Invalid metric value") end;
    def attrs: map({key, value: (.value | to_entries[0].value)}) | from_entries;
    reduce (inputs | .resourceMetrics[]? as $resource
      | $resource.scopeMetrics[]? as $scope | $scope.metrics[]?
      | . as $metric | select(.name == "HEALTH_METRIC" or ($limits[0] | has($metric.name)))
      | if .sum == null and .gauge == null then error("Only counters and gauges supported: " + .name) else . end
      | (.sum // .gauge).dataPoints[]? | select(((.flags // 0) % 2) == 0)
      | {name: $metric.name, value: ((.asInt // .asDouble) | numeric),
         attributes: ((.attributes // []) | attrs),
         resource: (($resource.resource.attributes // []) | attrs), scope: ($scope.scope // {})}
    ) as $point ({health: 0, errors: []};
      if $point.name == "HEALTH_METRIC" then .health = ([.health, $point.value] | max) else . end
      | if $phase == "final" and ($limits[0] | has($point.name)) and $point.value > $limits[0][$point.name]
        then .errors += ["\($point.name)=\($point.value); maximum=\($limits[0][$point.name]); attributes=\($point.attributes | tojson); resource=\($point.resource | tojson); scope=\($point.scope | tojson)"]
        else . end)
    | if (.errors | length) > 0 then
        .errors | unique | map(gsub("%"; "%25") | gsub("\r"; "%0D") | gsub("\n"; "%0A") | "::error::" + .)
        | join("\n") + "\n" | halt_error(1)
      else .health end
  '
}

snapshot_metrics() {
  # Ignore only an unfinished trailing JSONL record while the collector writes.
  cp "$1" "$2.raw" || return 1
  if [[ $(tail -c 1 "$2.raw" | od -An -tu1 | tr -d '[:space:]') == 10 ]]; then
    cp "$2.raw" "$2"
  else
    sed '$d' "$2.raw" >"$2"
  fi
}

watch_metrics() {
  local config_file="$1" metrics_file="$2" record="" chunk=""
  while [[ ! -r "$metrics_file" ]]; do sleep 1; done
  # The exporter never rotates this file. Keep the descriptor open so each
  # complete JSONL record is checked once, without rescanning earlier exports.
  exec 3<"$metrics_file"
  while :; do
    chunk=""
    if IFS= read -r chunk <&3; then
      record+="$chunk"
      parse_metrics "$config_file" final <<<"$record" >/dev/null || return 1
      record=""
    else
      record+="$chunk"
      sleep 1
    fi
  done
}

main() (
  local metrics_file="${CI_METRICS_FILE:-$metrics_script_dir/../ci-metrics/metrics.jsonl}"
  local health_url="http://localhost:8080/health"
  local config_file="$metrics_script_dir/../config/ci-metrics.json"
  local timeout=30 collector_filter=false watch=false
  while (( $# )); do
    if [[ "$1" == --collector-filter ]]; then
      collector_filter=true; shift; continue
    fi
    if [[ "$1" == --watch ]]; then
      watch=true; shift; continue
    fi
    if (( $# < 2 )); then
      echo "Usage: $0 [--config FILE] [--collector-filter|--watch] [--metrics-file FILE] [--health-url URL] [--timeout SECONDS]" >&2
      return 1
    fi
    case "$1" in
      --config) config_file="$2" ;;
      --metrics-file) metrics_file="$2" ;;
      --health-url) health_url="$2" ;;
      --timeout) timeout="$2" ;;
      *) echo "Unknown argument: $1" >&2; return 1 ;;
    esac
    shift 2
  done
  if ! jq -se 'length == 1 and (.[0] | type == "object" and length > 0 and
      all(to_entries[]; (.key | test("^[a-zA-Z_][a-zA-Z0-9_.:-]*$")) and
        (.value | type == "number" and isfinite and (isnan | not))))' "$config_file" >/dev/null; then
    echo 'ERROR: Config must be a nonempty JSON object mapping metric names to numeric maximums' >&2
    return 1
  fi
  if [[ "$collector_filter" == true ]]; then
    # OTTL drops metrics matching this condition; retain configured names + health.
    jq -r '(keys + ["HEALTH_METRIC"] | unique | map("name != " + tojson)) | join(" and ")' "$config_file"
    return
  fi
  if [[ "$watch" == true ]]; then
    watch_metrics "$config_file" "$metrics_file"
    return
  fi
  if [[ ! "$timeout" =~ ^[1-9][0-9]*$ ]]; then
    echo "Timeout must be a positive integer" >&2
    return 1
  fi

  local work_dir
  work_dir=$(mktemp -d)
  trap 'rm -rf "$work_dir"' EXIT
  local deadline=$((SECONDS + timeout)) baseline="" exported_at="" health remaining
  while (( SECONDS < deadline )); do
    if [[ -r "$metrics_file" ]] && snapshot_metrics "$metrics_file" "$work_dir/snapshot"; then
      health=$(parse_metrics "$config_file" probe <"$work_dir/snapshot") || return 1
      if [[ -z "$baseline" ]]; then
        baseline="$health"
        remaining=$((deadline - SECONDS))
        (( remaining > 0 )) || break
        # A fresh health increment proves the post-test JSON pipeline is alive.
        if ! curl --fail --silent --show-error --max-time "$remaining" "$health_url" >/dev/null; then baseline=""; fi
      elif (( health > baseline )); then
        if [[ -z "$exported_at" ]]; then exported_at=$SECONDS; fi
        # Two 3s application export cycles plus the collectors 1s file flush.
        if (( SECONDS - exported_at >= 7 )); then
          parse_metrics "$config_file" final <"$work_dir/snapshot" >/dev/null || return 1
          echo "All CI metric checks passed; telemetry verified."
          return 0
        fi
      fi
    else
      baseline=""; exported_at=""
    fi
    sleep 1
  done
  echo "ERROR: Could not verify application telemetry within ${timeout}s: $metrics_file" >&2
  return 1
)

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then main "$@"; fi
