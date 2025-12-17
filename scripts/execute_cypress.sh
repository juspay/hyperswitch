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
  # Added xvfb-run to the list
  local dependencies=("parallel" "npm" "xvfb-run")
  
  for cmd in "${dependencies[@]}"; do
    if ! command_exists "$cmd"; then
      print_color "red" "ERROR: ${cmd} is not installed!"
      # Suggest installation for Ubuntu/Debian users
      print_color "yellow" "Tip: Try running 'sudo apt-get install xvfb' if you are on Linux."
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

  local start_time=$(date +%s)
  
  echo "----------------------------------------------------"
  print_color "blue" "[START] Service: $service | Connector: $connector"
  echo "----------------------------------------------------"

  export REPORT_NAME="${service}_${connector}_report"
  export CYPRESS_CONNECTOR="$connector"

  # FIX: Use xvfb-run to assign a unique display number automatically
  # --auto-servernum : Finds a free display number (e.g., 99, 100, 101)
  # --server-args    : Sets screen dimensions to avoid rendering issues
  if xvfb-run --auto-servernum --server-args="-screen 0 1280x1024x24" npm run "cypress:$service"; then
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    print_color "green" "[PASS] $service:$connector (Time: ${duration}s)"
    return 0
  else
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    print_color "red" "[FAIL] $service:$connector (Time: ${duration}s)"
    
    echo "${service}:${connector}" >> "$failure_log"
    return 1
  fi
}
export -f execute_test

run_tests() {
  local jobs="${1:-1}"
  
  # Load Env Vars
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

  # 1. Loop through services
  for key in "${!service_map[@]}"; do
    declare -n connectors="$key"

    # 2. Case: Service has NO connectors (Standalone test)
    if [[ ${#connectors[@]} -eq 0 ]]; then
      local run_service="${service_map[$key]}"
      # Fix naming convention if needed (e.g. payment_method_list -> payment-method-list)
      [[ $run_service == "payment_method_list" ]] && run_service="payment-method-list"

      print_color "yellow" "Running standalone service: ${run_service}"
      export REPORT_NAME="${run_service}_report"
      
      if ! npm run "cypress:${run_service}"; then
        echo "${run_service}" >> "${tmp_file}"
      fi

    # 3. Case: Service HAS connectors (Parallel execution)
    else
      print_color "yellow" "🚀 Starting Parallel Execution for '${service_map[$key]}' (Jobs: $jobs)"
      
      parallel --jobs "$jobs" \
               --group \
               --joblog "$job_log" \
               execute_test {} "${service_map[$key]}" "${tmp_file}" ::: "${connectors[@]}" || true
    fi
  done

  # ==========================================================
  # FINAL SUMMARY REPORT
  # ==========================================================
  if [[ -s "${tmp_file}" ]]; then
    print_color "red" "\n========================================"
    print_color "red" "❌  TEST FAILURE SUMMARY"
    print_color "red" "========================================"
    print_color "red" "The following connectors failed:"
    
    # Read the file, sort unique entries, and print nicely
    sort -u "${tmp_file}" | while read -r line; do
       echo "  • $line"
    done
    
    print_color "red" "========================================"
    return 1  # Exit with failure
  else
    print_color "green" "\n========================================"
    print_color "green" "✅  SUCCESS: All tests passed!"
    print_color "green" "========================================"
    return 0  # Exit with success
  fi
}

# -----------------------------
# Main
# -----------------------------
main() {
  local command="${1:-}"
  local jobs="${2:-3}" 
  local test_dir="${3:-cypress-tests}"

  # Navigate to directory
  if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" ]]; then
    if [[ -d "$test_dir" ]]; then
      cd "${test_dir}"
    fi
  fi

  check_dependencies
  
  case "$command" in
    --parallel|-p)
      run_tests "$jobs"
      ;;
    *)
      run_tests 1
      ;;
  esac
}

main "$@"