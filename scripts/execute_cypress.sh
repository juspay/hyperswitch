#!/bin/bash

set -x
# Exit immediately if a command exits with a non-zero status,
# Treat unset variables as an error, and prevent errors in a pipeline from being masked
set -euo pipefail

# Define arrays for services, etc.
# Read service arrays from environment variables
read -r -a payments <<< "${PAYMENTS[@]:-}"
read -r -a payouts <<< "${PAYOUTS[@]:-}"
read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST[@]:-}"
read -r -a routing <<< "${ROUTING[@]:-}"

# define arrays
connector_map=()
failed_connectors=()

# Define an associative array to map environment variables to service names
declare -A services=(
  ["PAYMENTS"]="payments"
  ["PAYOUTS"]="payouts"
  ["PAYMENT_METHOD_LIST"]="payment_method_list"
  ["ROUTING"]="routing"
)

# Function to print messages in color
function print_color() {
  # Input params
  local color="$1"
  local message="$2"

  # Define colors
  local reset='\033[0m'
  local red='\033[0;31m'
  local green='\033[0;32m'
  local yellow='\033[0;33m'

  # Use indirect reference to get the color value
  echo -e "${!color}${message}${reset}"
}
export -f print_color

# Function to check if a command exists
function command_exists() {
  command -v "$1" > /dev/null 2>&1
}

# Function to read service arrays from environment variables
function read_service_arrays() {
  # Loop through the associative array and check if each service is exported
  for var in "${!services[@]}"; do
    if [[ -n "${!var+x}" ]]; then
      connector_map+=("${services[$var]}")
    fi
  done
}

# Function to execute Cypress tests
function execute_test() {
  if [[ $# -lt 3 ]]; then
    print_color "red" "ERROR: Insufficient arguments provided to execute_test."
    exit 1
  fi

  local connector="$1"
  local service="$2"
  local tmp_file="$3"

  print_color "yellow" "Executing tests for ${service} with connector ${connector}..."

  export REPORT_NAME="${service}_${connector}_report"

  if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
    echo "${service}-${connector}" >> "${tmp_file}"
  fi
}
export -f execute_test

# Function to run tests
function run_tests() {
  local jobs="${1:-1}"
  local tmp_file=$(mktemp)

  # Ensure temporary file is removed on script exit
  trap 'rm -f "${tmp_file}"' EXIT

  for service in "${connector_map[@]}"; do
    # Use indirect reference to get the array by service name
    declare -n connectors="$service"

    if [[ ${#connectors[@]} -eq 0 ]]; then
      # Service-level test (e.g., payment-method-list or routing)
      [[ $service == "payment_method_list" ]] && service="payment-method-list"

      echo "Running ${service} tests without connectors..."
      export REPORT_NAME="${service}_report"

      if ! npm run "cypress:${service}"; then
        echo "${service}" >> "${tmp_file}"
      fi
    else
      # Connector-specific tests (e.g., payments or payouts)
      print_color "yellow" "Running tests for service: '${service}' with connectors: [${connectors[*]}] in batches of ${jobs}..."

      # Execute tests in parallel
      printf '%s\n' "${connectors[@]}" | parallel --jobs "${jobs}" execute_test {} "${service}" "${tmp_file}"
    fi
  done

  # Collect failed connectors
  if [[ -s "${tmp_file}" ]]; then
    failed_connectors=($(< "${tmp_file}"))
    print_color "red" "One or more connectors failed to run:"
    printf '%s\n' "${failed_connectors[@]}"
    exit 1
  else
    print_color "green" "Cypress tests execution successful!"
  fi

}

# Function to check and install dependencies
function check_dependencies() {
  # parallel and npm are mandatory dependencies. exit the script if not found.
  local dependencies=("parallel" "npm")

  for cmd in "${dependencies[@]}"; do
    if ! command_exists "$cmd"; then
      print_color "red" "ERROR: ${cmd^} is not installed!"
      exit 1
    else
      print_color "green" "${cmd^} is installed already!"

      if [[ ${cmd} == "npm" ]]; then
        npm ci || {
          print_color "red" "Command \`npm ci\` failed!"
          exit 1
        }
      fi
    fi
  done
}

function cleanup() {
  print_color "yellow" "Cleaning up..."
  cd -
  unset PAYMENTS PAYOUTS PAYMENT_METHOD_LIST ROUTING
}

# Main function
function main() {
  local command="${1:-}"
  local jobs="${2:-4}"

  # Ensure script runs from 'cypress-tests' directory
  if [[ "$(basename "$PWD")" != "cypress-tests" ]]; then
    print_color "yellow" "Changing directory to 'cypress-tests'..."
    cd cypress-tests || {
      print_color "red" "ERROR: Directory 'cypress-tests' not found!"
      exit 1
    }
  fi

  check_dependencies
  read_service_arrays

  case "$command" in
    --parallel | -p)
      print_color "yellow" "WARNING: Running Cypress tests in parallel is more resource-intensive!"
      # At present, parallel execution is limited to batch of 4 to not run out of memory
      run_tests "$jobs"
      ;;
    *)
      run_tests 1
      ;;
  esac

  cleanup
}

# Execute the main function with passed arguments
main "$@"

set +x
