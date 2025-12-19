# #!/usr/bin/env bash

# set -euo pipefail

# # -----------------------------
# # Global Variables & Cleanup
# # -----------------------------
# tmp_file=$(mktemp)
# job_log=$(mktemp) 

# cleanup() {
#   local exit_code=$?
#   if [[ -f "${tmp_file}" ]]; then rm -f "${tmp_file}"; fi
#   if [[ -f "${job_log}" ]]; then rm -f "${job_log}"; fi
  
#   # Kill any stray Xvfb processes owned by this script
#   pkill -P $$ Xvfb 2>/dev/null || true
#   exit "$exit_code"
# }
# trap cleanup EXIT

# # -----------------------------
# # Helper Functions
# # -----------------------------
# print_color() {
#   local color_name="$1"
#   local message="$2"
#   local reset='\033[0m'
#   local red='\033[0;31m'
#   local green='\033[0;32m'
#   local yellow='\033[0;33m'
#   local blue='\033[0;34m'
  
#   case "$color_name" in
#     red) echo -e "${red}${message}${reset}" ;;
#     green) echo -e "${green}${message}${reset}" ;;
#     yellow) echo -e "${yellow}${message}${reset}" ;;
#     blue) echo -e "${blue}${message}${reset}" ;;
#     *) echo -e "${message}" ;;
#   esac
# }
# export -f print_color

# command_exists() {
#   command -v "$1" > /dev/null 2>&1
# }

# check_dependencies() {
#   local dependencies=("parallel" "npm")
#   for cmd in "${dependencies[@]}"; do
#     if ! command_exists "$cmd"; then
#       print_color "red" "ERROR: ${cmd} is not installed!"
#       exit 1
#     fi
#   done
  
#   if [[ ! -d "node_modules" ]]; then
#      print_color "yellow" "Installing NPM dependencies..."
#      npm ci
#   fi

#   # Verify Cypress ONCE (Verbose)
#   print_color "blue" "Verifying Cypress binary..."
  
#   if command_exists "xvfb-run"; then
#     xvfb-run --auto-servernum npm exec cypress verify
#   else
#     export DISPLAY=:99
#     Xvfb :99 &
#     local xvfb_pid=$!
    
#     # Run verify
#     npm exec cypress verify
    
#     kill $xvfb_pid 2>/dev/null || true
#   fi
#   print_color "green" "Cypress verified."
# }

# # -----------------------------
# # Test Execution Logic
# # -----------------------------
# execute_test() {
#   local connector="$1"
#   local service="$2"
#   local failure_log="$3"
#   local job_slot="${4:-1}" 

#   # --- CAPTURE START TIME ---
#   local start_ts=$(date +%s)
#   local start_fmt=$(date '+%H:%M:%S')
  
#   echo "----------------------------------------------------"
#   print_color "blue" "[START] $service:$connector (Slot: $job_slot) at $start_fmt"
#   echo "----------------------------------------------------"

#   export REPORT_NAME="${service}_${connector}_report"

#   # MANUALLY HANDLE XVFB
#   local unique_display=$((100 + job_slot))
#   export DISPLAY=":${unique_display}"
  
#   Xvfb "$DISPLAY" &
#   local xvfb_pid=$!
#   trap "kill $xvfb_pid 2>/dev/null || true" RETURN
#   sleep 1

#   local exit_code=0
  
#   if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
#     exit_code=1
#   fi

#   # --- CAPTURE END TIME ---
#   local end_ts=$(date +%s)
#   local end_fmt=$(date '+%H:%M:%S')
#   local duration=$((end_ts - start_ts))

#   if [[ $exit_code -eq 0 ]]; then
#     print_color "green" "[PASS] $service:$connector | Time: $start_fmt - $end_fmt (${duration}s)"
#     return 0
#   else
#     print_color "red" "[FAIL] $service:$connector | Time: $start_fmt - $end_fmt (${duration}s)"
#     echo "${service}:${connector}" >> "$failure_log"
#     return 1
#   fi
# }
# export -f execute_test
# export -f command_exists 
# export -f print_color    

# run_tests() {
#   local jobs="${1:-1}"
  
#   read -r -a payments <<< "${PAYMENTS_CONNECTORS:-}"
#   read -r -a payouts <<< "${PAYOUTS_CONNECTORS:-}"
#   read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST:-}"
#   read -r -a routing <<< "${ROUTING:-}"

#   declare -A env_to_service=(
#     ["PAYMENTS_CONNECTORS"]="payments"
#     ["PAYOUTS_CONNECTORS"]="payouts"
#     ["PAYMENT_METHOD_LIST"]="payment_method_list"
#     ["ROUTING"]="routing"
#   )

#   # Filter Active Services
#   local active_services=()
#   for env_var in "${!env_to_service[@]}"; do
#     if [[ -n "${!env_var:-}" ]]; then
#       active_services+=("${env_to_service[$env_var]}")
#     fi
#   done

#   for service in "${active_services[@]}"; do
#     declare -n connectors="$service"

#     if [[ ${#connectors[@]} -gt 0 ]]; then
#       print_color "yellow" "🚀 Starting Parallel Execution for '$service' (Jobs: $jobs)"
      
#       parallel --jobs "$jobs" \
#                --group \
#                --joblog "$job_log" \
#                execute_test {} "$service" "${tmp_file}" {%} ::: "${connectors[@]}" || true

#     else
#       local run_service="$service"
#       [[ $run_service == "payment_method_list" ]] && run_service="payment-method-list"

#       print_color "yellow" "Running standalone service: ${run_service}"
#       export REPORT_NAME="${run_service}_report"
      
#       export DISPLAY=:99
#       Xvfb :99 & 
#       local pid=$!
#       sleep 1
      
#       # Standalone Timing
#       local s_start=$(date +%s)
#       local s_start_fmt=$(date '+%H:%M:%S')
#       print_color "blue" "[START] $run_service at $s_start_fmt"

#       if npm run "cypress:${run_service}"; then
#          local s_end=$(date +%s)
#          local s_end_fmt=$(date '+%H:%M:%S')
#          local s_dur=$((s_end - s_start))
#          print_color "green" "[PASS] $run_service | Time: $s_start_fmt - $s_end_fmt (${s_dur}s)"
#       else
#          local s_end=$(date +%s)
#          local s_end_fmt=$(date '+%H:%M:%S')
#          local s_dur=$((s_end - s_start))
#          print_color "red" "[FAIL] $run_service | Time: $s_start_fmt - $s_end_fmt (${s_dur}s)"
#          echo "${run_service}" >> "${tmp_file}"
#       fi
#       kill $pid 2>/dev/null || true
#     fi
#   done

#   if [[ -s "${tmp_file}" ]]; then
#     print_color "red" "\n========================================"
#     print_color "red" "❌  TEST FAILURE SUMMARY"
#     print_color "red" "========================================"
#     sort -u "${tmp_file}" | while read -r line; do echo "  • $line"; done
#     print_color "red" "========================================"
#     return 1
#   else
#     print_color "green" "\n✅  SUCCESS: All tests passed!"
#     return 0
#   fi
# }

# main() {
#   local command="${1:-}"
#   local jobs="${2:-3}" 
#   local test_dir="${3:-cypress-tests}"

#   if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" && -d "$test_dir" ]]; then
#     cd "${test_dir}"
#   fi

#   check_dependencies
  
#   case "$command" in
#     --parallel|-p) run_tests "$jobs" ;;
#     *) run_tests 1 ;;
#   esac
# }

# main "$@"

# #! /usr/bin/env bash

# set -euo pipefail

# # Initialize tmp_file globally
# tmp_file=""

# # Define arrays for services, etc.
# # Read service arrays from environment variables
# read -r -a payments <<< "${PAYMENTS_CONNECTORS[@]:-}"
# read -r -a payouts <<< "${PAYOUTS_CONNECTORS[@]:-}"
# read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST[@]:-}"
# read -r -a routing <<< "${ROUTING[@]:-}"

# # Define arrays
# connector_map=()
# failed_connectors=()

# # Define an associative array to map environment variables to service names
# declare -A services=(
#   ["PAYMENTS_CONNECTORS"]="payments"
#   ["PAYOUTS_CONNECTORS"]="payouts"
#   ["PAYMENT_METHOD_LIST"]="payment_method_list"
#   ["ROUTING"]="routing"
# )

# # Function to print messages in color
# function print_color() {
#   # Input params
#   local color="$1"
#   local message="$2"

#   # Define colors
#   local reset='\033[0m'
#   local red='\033[0;31m'
#   local green='\033[0;32m'
#   local yellow='\033[0;33m'

#   # Use indirect reference to get the color value
#   echo -e "${!color}${message}${reset}"
# }
# export -f print_color

# # Function to check if a command exists
# function command_exists() {
#   command -v "$1" > /dev/null 2>&1
# }

# # Function to read service arrays from environment variables
# function read_service_arrays() {
#   # Loop through the associative array and check if each service is exported
#   for var in "${!services[@]}"; do
#     if [[ -n "${!var+x}" ]]; then
#       connector_map+=("${services[$var]}")
#     else
#       print_color "yellow" "Environment variable ${var} is not set. Skipping..."
#     fi
#   done
# }

# # Function to execute Cypress tests
# function execute_test() {
#   if [[ $# -lt 3 ]]; then
#     print_color "red" "ERROR: Insufficient arguments provided to execute_test."
#     exit 1
#   fi

#   local connector="$1"
#   local service="$2"
#   local tmp_file="$3"

#   print_color "yellow" "Executing tests for ${service} with connector ${connector}..."

#   export REPORT_NAME="${service}_${connector}_report"

#   if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
#     echo "${service}-${connector}" >> "${tmp_file}"
#   fi
# }
# export -f execute_test

# # Function to run tests
# function run_tests() {
#   local jobs="${1:-1}"
#   tmp_file=$(mktemp)

#   # Ensure temporary file is removed on script exit
#   trap 'cleanup' EXIT

#   for service in "${connector_map[@]}"; do
#     declare -n connectors="$service"

#     if [[ ${#connectors[@]} -eq 0 ]]; then
#       # Service-level test (e.g., payment-method-list or routing)
#       [[ $service == "payment_method_list" ]] && service="payment-method-list"

#       echo "Running ${service} tests without connectors..."
#       export REPORT_NAME="${service}_report"

#       if ! npm run "cypress:${service}"; then
#         echo "${service}" >> "${tmp_file}"
#       fi
#     else
#       # Connector-specific tests (e.g., payments or payouts)
#       print_color "yellow" "Running tests for service: '${service}' with connectors: [${connectors[*]}] in batches of ${jobs}..."

#       # Execute tests in parallel
#       printf '%s\n' "${connectors[@]}" | parallel --jobs "${jobs}" execute_test {} "${service}" "${tmp_file}"
#     fi
#   done

#   # Collect failed connectors
#   if [[ -s "${tmp_file}" ]]; then
#     failed_connectors=($(< "${tmp_file}"))
#     print_color "red" "One or more connectors failed to run:"
#     printf '%s\n' "${failed_connectors[@]}"
#     exit 1
#   else
#     print_color "green" "Cypress tests execution successful!"
#   fi
# }

# # Function to check and install dependencies
# function check_dependencies() {
#   # parallel and npm are mandatory dependencies. exit the script if not found.
#   local dependencies=("parallel" "npm")

#   for cmd in "${dependencies[@]}"; do
#     if ! command_exists "$cmd"; then
#       print_color "red" "ERROR: ${cmd^} is not installed!"
#       exit 1
#     else
#       print_color "green" "${cmd^} is installed already!"

#       if [[ ${cmd} == "npm" ]]; then
#         npm ci || {
#           print_color "red" "Command \`npm ci\` failed!"
#           exit 1
#         }
#       fi
#     fi
#   done
# }

# # Cleanup function to handle exit
# function cleanup() {
#   print_color "yellow" "Cleaning up..."
#   if [[ -d "cypress-tests" ]]; then
#     cd -
#   fi

#   if [[ -n "${tmp_file}" && -f "${tmp_file}" ]]; then
#     rm -f "${tmp_file}"
#   fi
# }

# # Main function
# function main() {
#   local command="${1:-}"
#   local jobs="${2:-5}"
#   local test_dir="${3:-cypress-tests}"

#   # Ensure script runs from the specified test directory (default: cypress-tests)
#   if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" ]]; then
#     print_color "yellow" "Changing directory to '${test_dir}'..."
#     cd "${test_dir}" || {
#       print_color "red" "ERROR: Directory '${test_dir}' not found!"
#       exit 1
#     }
#   fi

#   check_dependencies
#   read_service_arrays

#   case "$command" in
#     --parallel | -p)
#       print_color "yellow" "WARNING: Running Cypress tests in parallel is more resource-intensive!"
#       # At present, parallel execution is restricted to not run out of memory
#       # But can be scaled up by passing the value as an argument
#       run_tests "$jobs"
#       ;;
#     *)
#       run_tests 1
#       ;;
#   esac
# }

# # Execute the main function with passed arguments
# main "$@"

#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# Global setup
# -----------------------------
TMP_FAILURES=$(mktemp)
JOB_LOG=$(mktemp)

cleanup() {
  local exit_code=$?
  rm -f "$TMP_FAILURES" "$JOB_LOG"
  pkill -P $$ Xvfb 2>/dev/null || true
  exit "$exit_code"
}
trap cleanup EXIT

# -----------------------------
# Helpers
# -----------------------------
print_color() {
  local color="$1"; shift
  local reset='\033[0m'
  local red='\033[0;31m'
  local green='\033[0;32m'
  local yellow='\033[0;33m'
  local blue='\033[0;34m'

  case "$color" in
    red) echo -e "${red}$*${reset}" ;;
    green) echo -e "${green}$*${reset}" ;;
    yellow) echo -e "${yellow}$*${reset}" ;;
    blue) echo -e "${blue}$*${reset}" ;;
    *) echo "$*" ;;
  esac
}
export -f print_color

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# -----------------------------
# Dependency checks
# -----------------------------
check_dependencies() {
  local deps=(parallel npm xvfb-run taskset)

  for d in "${deps[@]}"; do
    if ! command_exists "$d"; then
      print_color red "ERROR: Missing dependency: $d"
      exit 1
    fi
  done

  if [[ ! -d node_modules ]]; then
    print_color yellow "Installing npm dependencies..."
    npm ci
  fi

  print_color blue "Verifying Cypress binary..."
  xvfb-run -a npm exec cypress verify
  print_color green "Cypress verified"
}

# -----------------------------
# Test execution (ONE connector)
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local failure_log="$3"
  local slot="${4:-1}"

  local start_ts=$(date +%s)
  local start_fmt=$(date '+%H:%M:%S')

  print_color blue "[START] $service:$connector | Slot $slot | $start_fmt"

  # ---- CPU PINNING ----
  local CORES_PER_CONNECTOR=2
  local START_CORE=$(((slot - 1) * CORES_PER_CONNECTOR))
  local END_CORE=$((START_CORE + CORES_PER_CONNECTOR - 1))

  export REPORT_NAME="${service}_${connector}_report"

  if ! taskset -c "${START_CORE}-${END_CORE}" \
        xvfb-run -a \
        bash -c '
          export CYPRESS_CONNECTORS="'"$connector"'"
          npm run "cypress:'"$service"'"
        '
  then
    print_color red "[FAIL] $service:$connector"
    echo "${service}:${connector}" >> "$failure_log"
    return 1
  fi

  local end_ts=$(date +%s)
  local duration=$((end_ts - start_ts))
  print_color green "[PASS] $service:$connector | ${duration}s"
}
export -f execute_test

# -----------------------------
# Runner
# -----------------------------
run_tests() {
  local TOTAL_CORES
  TOTAL_CORES=$(nproc)

  local CORES_PER_CONNECTOR=2
  local MAX_JOBS=$((TOTAL_CORES / CORES_PER_CONNECTOR))

  print_color yellow "Detected cores: $TOTAL_CORES"
  print_color yellow "Running with $MAX_JOBS parallel connectors"

  # Reduce disk pressure
  export CYPRESS_VIDEO=false
  export CYPRESS_SCREENSHOTS_FOLDER=/tmp/cypress-screenshots
  export CYPRESS_VIDEOS_FOLDER=/tmp/cypress-videos

  # Services map
  declare -A SERVICES=(
    ["PAYMENTS_CONNECTORS"]="payments"
    ["PAYOUTS_CONNECTORS"]="payouts"
    ["PAYMENT_METHOD_LIST"]="payment-method-list"
    ["ROUTING"]="routing"
  )

  for env_var in "${!SERVICES[@]}"; do
    [[ -z "${!env_var:-}" ]] && continue

    read -r -a connectors <<< "${!env_var}"
    local service="${SERVICES[$env_var]}"

    print_color yellow "🚀 Running $service (${#connectors[@]} connectors)"

    parallel \
      --jobs "$MAX_JOBS" \
      --halt soon,fail=1 \
      --line-buffer \
      --joblog "$JOB_LOG" \
      execute_test {} "$service" "$TMP_FAILURES" {%} \
      ::: "${connectors[@]}" || true
  done

  if [[ -s "$TMP_FAILURES" ]]; then
    print_color red "\n❌ FAILURE SUMMARY"
    sort -u "$TMP_FAILURES" | sed 's/^/  • /'
    exit 1
  fi

  print_color green "\n✅ ALL TESTS PASSED"
}

# -----------------------------
# Main
# -----------------------------
main() {
  check_dependencies
  run_tests
}

main "$@"
