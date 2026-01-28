#!/usr/bin/env bash
# Hardened Cypress Parallel Execution Script
set -euo pipefail

# -----------------------------
# Global Variables & Cleanup
# -----------------------------
tmp_file=$(mktemp)        # failure-only tracking
job_log=$(mktemp)         # GNU parallel job log
results_log=$(mktemp)     # unified results log
lock_file="/tmp/cypress_results.lock"

cleanup() {
  local exit_code=$?
  print_color yellow "Cleaning up temporary files and stray processes..."
  
  # Kill Cypress, Chrome, and Xvfb specifically to prevent memory zombies
  pkill -9 -f "cypress" 2>/dev/null || true
  pkill -9 -f "chrome" 2>/dev/null || true
  pkill -9 -f "Xvfb" 2>/dev/null || true
  
  [[ -f "$tmp_file" ]] && rm -f "$tmp_file"
  [[ -f "$job_log" ]] && rm -f "$job_log"
  [[ -f "$results_log" ]] && rm -f "$results_log"
  [[ -f "$lock_file" ]] && rm -f "$lock_file"

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
export -f command_exists

# -----------------------------
# Dependency Check
# -----------------------------
check_dependencies() {
  local dependencies=("parallel" "npm" "xvfb-run")
  for cmd in "${dependencies[@]}"; do
    if ! command_exists "$cmd"; then
      print_color red "ERROR: ${cmd} is not installed! (Run: sudo apt-get install parallel xvfb)"
      exit 1
    fi
  done

  if [[ ! -d "node_modules" ]]; then
    print_color yellow "Installing NPM dependencies..."
    npm ci
  fi

  print_color blue "Verifying Cypress binary..."
  xvfb-run --auto-servernum npm exec cypress verify
  print_color green "Cypress verified."
}

# -----------------------------
# Test Execution Logic
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local failure_log="$3"
  local res_log="$4"
  local lock_fn="$5"
  local job_slot="${6:-1}"

  local start_ts=$(date +%s)
  local start_fmt=$(date '+%H:%M:%S')

  echo "----------------------------------------------------"
  print_color blue "[START] $service:$connector (Slot: $job_slot) at $start_fmt"
  echo "----------------------------------------------------"

  # Set specific environment variables
  export CYPRESS_CONNECTOR="$connector"
  export REPORT_NAME="${service}_${connector}_report"

  # Execute Cypress via xvfb-run
  # --auto-servernum: Prevents "Display already in use" race conditions
  # --server-args: Ensures a standard screen size for snapshots
  local exit_code=0
  if ! xvfb-run --auto-servernum --server-args="-screen 0 1280x1024x24" \
       npm run "cypress:${service}"; then
    exit_code=1
  fi

  local duration=$(($(date +%s) - start_ts))
  local status="PASS"
  [[ $exit_code -ne 0 ]] && status="FAIL"

  # Atomically record result using flock to prevent interleaved lines
  (
    flock -x 200
    echo "${service}:${connector}:${status}:${duration}" >> "$res_log"
    if [[ "$status" == "FAIL" ]]; then
      echo "${service}:${connector}" >> "$failure_log"
      print_color red "[FAIL] $service:$connector | ${duration}s"
    else
      print_color green "[PASS] $service:$connector | ${duration}s"
    fi
  ) 200>"$lock_fn"

  return $exit_code
}

export -f execute_test

# -----------------------------
# Runner Logic
# -----------------------------
run_tests() {
  local jobs="${1:-1}"

  # Build arrays from environment variables
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

  for env_var in "${!env_to_service[@]}"; do
    local service="${env_to_service[$env_var]}"
    declare -n connectors="$service"

    if [[ ${#connectors[@]} -gt 0 ]]; then
      print_color yellow ">>> Starting Parallel Execution for '$service' (Workers: $jobs)"

      # Run parallel: Passing all logs as arguments to ensure subshell visibility
      parallel --jobs "$jobs" \
               --group \
               --joblog "$job_log" \
               execute_test {} "$service" "$tmp_file" "$results_log" "$lock_file" {%} \
               ::: "${connectors[@]}" || true
    fi
  done

  # -----------------------------
  # Final Summary Reporting
  # -----------------------------
  if [[ -s "$results_log" ]]; then
    echo -e "\n"
    print_color blue "================ EXECUTION SUMMARY ================"
    
    # PASS List
    awk -F: '$3=="PASS" { printf "\033[0;32m  ✔ %-30s | %4ss\033[0m\n", $1 ":" $2, $4 }' "$results_log"
    
    # FAIL List
    awk -F: '$3=="FAIL" { printf "\033[0;31m  ✖ %-30s | %4ss\033[0m\n", $1 ":" $2, $4 }' "$results_log"

    print_color blue "---------------------------------------------------"
    awk -F: '
    {
      count++; total += $4
      if ($3=="PASS") pass++
      if ($3=="FAIL") fail++
    }
    END {
      printf "  TOTAL: %d | SUCCESS: %d | FAILED: %d | AVG: %ds\n", count, pass, fail, (count?total/count:0)
    }' "$results_log"
    print_color blue "==================================================="
  fi

  # Exit 1 if the failure log has content
  [[ -s "$tmp_file" ]] && return 1 || return 0
}

# -----------------------------
# Main Entry Point
# -----------------------------
main() {
  local command="${1:-}"
  local jobs="${2:-3}"
  local test_dir="${3:-cypress-tests}"

  # Navigate to the correct test directory
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