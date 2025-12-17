#!/usr/bin/env bash

set -euo pipefail

# -----------------------------
# Global Variables & Cleanup
# -----------------------------
tmp_file=$(mktemp)
job_log=$(mktemp) 

# Cleanup function to remove temp files
cleanup() {
  local exit_code=$?
  if [[ -f "${tmp_file}" ]]; then rm -f "${tmp_file}"; fi
  if [[ -f "${job_log}" ]]; then rm -f "${job_log}"; fi
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
}

# -----------------------------
# Test Execution Logic
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local failure_log="$3"
  # Capture the Parallel Job Slot (unique number 1..N) to avoid port conflicts
  local job_slot="${4:-1}" 

  local start_time=$(date +%s)
  
  echo "----------------------------------------------------"
  print_color "blue" "[START] Service: $service | Connector: $connector (Slot: $job_slot)"
  echo "----------------------------------------------------"

  export REPORT_NAME="${service}_${connector}_report"

  # ---------------------------------------------------------
  # EXECUTION WITH DISPLAY HANDLING
  # ---------------------------------------------------------
  local exit_code=0

  if command_exists "xvfb-run"; then
    # OPTION A: xvfb-run is available (Standard Linux)
    # We prefix the env var strictly as requested: CYPRESS_CONNECTOR="..."
    # xvfb-run inherits the env var and passes it to npm
    if ! CYPRESS_CONNECTOR="$connector" xvfb-run --auto-servernum --server-args="-screen 0 1280x1024x24" npm run "cypress:$service"; then
      exit_code=1
    fi
  else
    # OPTION B: Manual Xvfb (Offline/Restricted Runners)
    # 1. Start Xvfb on a unique port calculated from the job slot (101, 102, etc.)
    local unique_display=$((100 + job_slot))
    export DISPLAY=":${unique_display}"
    
    print_color "yellow" "xvfb-run not found. Manually starting Xvfb on ${DISPLAY}..."
    Xvfb "$DISPLAY" -screen 0 1280x1024x24 &
    local xvfb_pid=$!
    
    # Ensure Xvfb is killed when this function returns
    trap "kill $xvfb_pid 2>/dev/null || true" RETURN
    sleep 2 # Give Xvfb a moment to initialize

    # 2. Run strictly using the requested syntax
    if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
      exit_code=1
    fi
  fi

  local end_time=$(date +%s)
  local duration=$((end_time - start_time))

  if [[ $exit_code -eq 0 ]]; then
    print_color "green" "[PASS] $service:$connector (Time: ${duration}s)"
    return 0
  else
    print_color "red" "[FAIL] $service:$connector (Time: ${duration}s)"
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

  declare -A service_map=(
    ["payments"]="payments"
    ["payouts"]="payouts"
    ["payment_method_list"]="payment_method_list"
    ["routing"]="routing"
  )

  for key in "${!service_map[@]}"; do
    declare -n connectors="$key"

    if [[ ${#connectors[@]} -eq 0 ]]; then
      local run_service="${service_map[$key]}"
      [[ $run_service == "payment_method_list" ]] && run_service="payment-method-list"

      print_color "yellow" "Running standalone service: ${run_service}"
      export REPORT_NAME="${run_service}_report"
      
      # Handle standalone execution with Xvfb awareness
      if command_exists "xvfb-run"; then
         xvfb-run --auto-servernum --server-args="-screen 0 1280x1024x24" npm run "cypress:${run_service}" || echo "${run_service}" >> "${tmp_file}"
      else
         export DISPLAY=:99
         Xvfb :99 -screen 0 1280x1024x24 & 
         local pid=$!
         npm run "cypress:${run_service}" || echo "${run_service}" >> "${tmp_file}"
         kill $pid 2>/dev/null || true
      fi

    else
      print_color "yellow" "🚀 Starting Parallel Execution for '${service_map[$key]}' (Jobs: $jobs)"
      
      # {%} passes the unique Job Slot ID to the function
      parallel --jobs "$jobs" \
               --group \
               --joblog "$job_log" \
               execute_test {} "${service_map[$key]}" "${tmp_file}" {%} ::: "${connectors[@]}" || true
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