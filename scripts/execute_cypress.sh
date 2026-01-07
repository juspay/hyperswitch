#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# Global Variables & Cleanup
# -----------------------------
tmp_file=$(mktemp)        # failure-only (legacy)
job_log=$(mktemp)         # GNU parallel job log
results_log=$(mktemp)     # unified results: service:connector:status:duration

cleanup() {
  local exit_code=$?
  [[ -f "$tmp_file" ]] && rm -f "$tmp_file"
  [[ -f "$job_log" ]] && rm -f "$job_log"
  [[ -f "$results_log" ]] && rm -f "$results_log"

  # Kill any stray Xvfb processes owned by this script
  pkill -P $$ Xvfb 2>/dev/null || true
  exit "$exit_code"
}
trap cleanup EXIT

# -----------------------------
# Helper Functions
# -----------------------------
print_color() {
  local color_name="$1"
  local message="$2"
  local reset='\033[0m'
  local red='\033[0;31m'
  local green='\033[0;32m'
  local yellow='\033[0;33m'
  local blue='\033[0;34m'

  case "$color_name" in
    red) echo -e "${red}${message}${reset}" ;;
    green) echo -e "${green}${message}${reset}" ;;
    yellow) echo -e "${yellow}${message}${reset}" ;;
    blue) echo -e "${blue}${message}${reset}" ;;
    *) echo -e "${message}" ;;
  esac
}
export -f print_color

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# -----------------------------
# Dependency Check
# -----------------------------
check_dependencies() {
  local dependencies=("parallel" "npm")
  for cmd in "${dependencies[@]}"; do
    if ! command_exists "$cmd"; then
      print_color red "ERROR: ${cmd} is not installed!"
      exit 1
    fi
  done

  if [[ ! -d "node_modules" ]]; then
    print_color yellow "Installing NPM dependencies..."
    npm ci
  fi

  print_color blue "Verifying Cypress binary..."
  if command_exists "xvfb-run"; then
    xvfb-run --auto-servernum npm exec cypress verify
  else
    export DISPLAY=:99
    Xvfb :99 &
    local xvfb_pid=$!
    npm exec cypress verify
    kill "$xvfb_pid" 2>/dev/null || true
  fi
  print_color green "Cypress verified."
}

# -----------------------------
# Test Execution Logic
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local failure_log="$3"
  local job_slot="${4:-1}"

  local start_ts
  start_ts=$(date +%s)
  local start_fmt
  start_fmt=$(date '+%H:%M:%S')

  echo "----------------------------------------------------"
  print_color blue "[START] $service:$connector (Slot: $job_slot) at $start_fmt"
  echo "----------------------------------------------------"

  export REPORT_NAME="${service}_${connector}_report"

  # -----------------------------
  # XVFB
  # -----------------------------
  local unique_display=$((100 + job_slot))
  export DISPLAY=":${unique_display}"

  Xvfb "$DISPLAY" &
  local xvfb_pid=$!
  trap "kill $xvfb_pid 2>/dev/null || true" RETURN
  sleep 1

  # -----------------------------
  # EXECUTE TEST
  # -----------------------------
  local exit_code=0
  if ! bash -c '
        export CYPRESS_CONNECTOR="'"$connector"'"
        npm run "cypress:'"$service"'"
      '
  then
    exit_code=1
  fi

  local end_ts
  end_ts=$(date +%s)
  local duration=$((end_ts - start_ts))

  local status
  if [[ $exit_code -eq 0 ]]; then
    status="PASS"
    print_color green "[PASS] $service:$connector | ${duration}s"
  else
    status="FAIL"
    print_color red "[FAIL] $service:$connector | ${duration}s"
  fi

  # Always record result
  echo "${service}:${connector}:${status}:${duration}" >> "$results_log"

  # Keep legacy failure list (used for exit code)
  if [[ "$status" == "FAIL" ]]; then
    echo "${service}:${connector}" >> "$failure_log"
    return 1
  fi

  return 0
}

export -f execute_test
export -f command_exists
export -f print_color
export results_log

# -----------------------------
# Runner
# -----------------------------
run_tests() {
  local jobs="${1:-1}"

  read -r -a payments <<< "${PAYMENTS_CONNECTORS:-}"
  read -r -a payouts <<< "${PAYOUTS_CONNECTORS:-}"
  read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST:-}"
  read -r -a routing <<< "${ROUTING:-}"

  declare -A env_to_service=(
    ["PAYMENTS_CONNECTORS"]="payments"
    ["PAYOUTS_CONNECTORS"]="payouts"
    ["PAYMENT_METHOD_LIST"]="payment_method_list"
    ["ROUTING"]="routing"
  )

  local active_services=()
  for env_var in "${!env_to_service[@]}"; do
    [[ -n "${!env_var:-}" ]] && active_services+=("${env_to_service[$env_var]}")
  done

  for service in "${active_services[@]}"; do
    declare -n connectors="$service"

    if [[ ${#connectors[@]} -gt 0 ]]; then
      print_color yellow "🚀 Starting Parallel Execution for '$service' (Jobs: $jobs)"

      parallel --jobs "$jobs" \
               --group \
               --joblog "$job_log" \
               execute_test {} "$service" "$tmp_file" {%} \
               ::: "${connectors[@]}" || true
    fi
  done

  # -----------------------------
  # Final Summary
  # -----------------------------
  if [[ -s "$results_log" ]]; then

    print_color green "\n========================================"
    print_color green " SUCCESSFUL CONNECTORS"
    print_color green "========================================"
    awk -F: '$3=="PASS" {
      printf "  • %-30s | %5ss\n", $1 ":" $2, $4
    }' "$results_log"
    print_color green "\n========================================"

    print_color red "\n========================================"
    print_color red " FAILED CONNECTORS"
    print_color red "========================================"
    awk -F: '$3=="FAIL" {
      printf "  • %-30s | %5ss\n", $1 ":" $2, $4
    }' "$results_log"
    print_color red "\n========================================"

    print_color blue "\n========================================"
    print_color blue " EXECUTION STATS"
    print_color blue "========================================"
    awk -F: '
    {
      total += $4
      count++
      if ($3=="PASS") pass++
      if ($3=="FAIL") fail++
    }
    END {
      printf "  ➜ Total connectors run: %d\n", count
      printf "  ➜ Successful: %d\n", pass
      printf "  ➜ Failed: %d\n", fail
      printf "  ➜ Total execution time: %ds\n", total
    }' "$results_log"
  fi

  [[ -s "$tmp_file" ]] && return 1 || return 0
}

# -----------------------------
# Main
# -----------------------------
main() {
  local command="${1:-}"
  local jobs="${2:-3}"
  local test_dir="${3:-cypress-tests}"

  if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" && -d "$test_dir" ]]; then
    cd "$test_dir"
  fi

  check_dependencies

  case "$command" in
    --parallel|-p) run_tests "$jobs" ;;
    *) run_tests 1 ;;
  esac
}

main "$@"
