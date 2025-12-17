#!/usr/bin/env bash

set -euo pipefail

# -----------------------------
# Global Variables & Cleanup
# -----------------------------
tmp_file=$(mktemp)
job_log=$(mktemp) 

cleanup() {
  local exit_code=$?
  if [[ -f "${tmp_file}" ]]; then rm -f "${tmp_file}"; fi
  if [[ -f "${job_log}" ]]; then rm -f "${job_log}"; fi
  
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
  command -v "$1" > /dev/null 2>&1
}

check_dependencies() {
  local dependencies=("parallel" "npm")
  for cmd in "${dependencies[@]}"; do
    if ! command_exists "$cmd"; then
      print_color "red" "ERROR: ${cmd} is not installed!"
      exit 1
    fi
  done
  
  if [[ ! -d "node_modules" ]]; then
     print_color "yellow" "Installing NPM dependencies..."
     npm ci
  fi

  # Verify Cypress ONCE (Verbose)
  print_color "blue" "Verifying Cypress binary..."
  
  if command_exists "xvfb-run"; then
    xvfb-run --auto-servernum npm exec cypress verify
  else
    export DISPLAY=:99
    Xvfb :99 &
    local xvfb_pid=$!
    
    # Run verify
    npm exec cypress verify
    
    kill $xvfb_pid 2>/dev/null || true
  fi
  print_color "green" "Cypress verified."
}

# -----------------------------
# Test Execution Logic
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local failure_log="$3"
  local job_slot="${4:-1}" 

  # --- CAPTURE START TIME ---
  local start_ts=$(date +%s)
  local start_fmt=$(date '+%H:%M:%S')
  
  echo "----------------------------------------------------"
  print_color "blue" "[START] $service:$connector (Slot: $job_slot) at $start_fmt"
  echo "----------------------------------------------------"

  export REPORT_NAME="${service}_${connector}_report"

  # MANUALLY HANDLE XVFB
  local unique_display=$((100 + job_slot))
  export DISPLAY=":${unique_display}"
  
  Xvfb "$DISPLAY" &
  local xvfb_pid=$!
  trap "kill $xvfb_pid 2>/dev/null || true" RETURN
  sleep 1

  local exit_code=0
  
  if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
    exit_code=1
  fi

  # --- CAPTURE END TIME ---
  local end_ts=$(date +%s)
  local end_fmt=$(date '+%H:%M:%S')
  local duration=$((end_ts - start_ts))

  if [[ $exit_code -eq 0 ]]; then
    print_color "green" "[PASS] $service:$connector | Time: $start_fmt - $end_fmt (${duration}s)"
    return 0
  else
    print_color "red" "[FAIL] $service:$connector | Time: $start_fmt - $end_fmt (${duration}s)"
    echo "${service}:${connector}" >> "$failure_log"
    return 1
  fi
}
export -f execute_test
export -f command_exists 
export -f print_color    

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

  # Filter Active Services
  local active_services=()
  for env_var in "${!env_to_service[@]}"; do
    if [[ -n "${!env_var:-}" ]]; then
      active_services+=("${env_to_service[$env_var]}")
    fi
  done

  for service in "${active_services[@]}"; do
    declare -n connectors="$service"

    if [[ ${#connectors[@]} -gt 0 ]]; then
      print_color "yellow" "🚀 Starting Parallel Execution for '$service' (Jobs: $jobs)"
      
      parallel --jobs "$jobs" \
               --group \
               --joblog "$job_log" \
               execute_test {} "$service" "${tmp_file}" {%} ::: "${connectors[@]}" || true

    else
      local run_service="$service"
      [[ $run_service == "payment_method_list" ]] && run_service="payment-method-list"

      print_color "yellow" "Running standalone service: ${run_service}"
      export REPORT_NAME="${run_service}_report"
      
      export DISPLAY=:99
      Xvfb :99 & 
      local pid=$!
      sleep 1
      
      # Standalone Timing
      local s_start=$(date +%s)
      local s_start_fmt=$(date '+%H:%M:%S')
      print_color "blue" "[START] $run_service at $s_start_fmt"

      if npm run "cypress:${run_service}"; then
         local s_end=$(date +%s)
         local s_end_fmt=$(date '+%H:%M:%S')
         local s_dur=$((s_end - s_start))
         print_color "green" "[PASS] $run_service | Time: $s_start_fmt - $s_end_fmt (${s_dur}s)"
      else
         local s_end=$(date +%s)
         local s_end_fmt=$(date '+%H:%M:%S')
         local s_dur=$((s_end - s_start))
         print_color "red" "[FAIL] $run_service | Time: $s_start_fmt - $s_end_fmt (${s_dur}s)"
         echo "${run_service}" >> "${tmp_file}"
      fi
      kill $pid 2>/dev/null || true
    fi
  done

  if [[ -s "${tmp_file}" ]]; then
    print_color "red" "\n========================================"
    print_color "red" "❌  TEST FAILURE SUMMARY"
    print_color "red" "========================================"
    sort -u "${tmp_file}" | while read -r line; do echo "  • $line"; done
    print_color "red" "========================================"
    return 1
  else
    print_color "green" "\n✅  SUCCESS: All tests passed!"
    return 0
  fi
}

main() {
  local command="${1:-}"
  local jobs="${2:-3}" 
  local test_dir="${3:-cypress-tests}"

  if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" && -d "$test_dir" ]]; then
    cd "${test_dir}"
  fi

  check_dependencies
  
  case "$command" in
    --parallel|-p) run_tests "$jobs" ;;
    *) run_tests 1 ;;
  esac
}

main "$@"