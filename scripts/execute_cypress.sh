#!/bin/bash
set -o nounset -euo pipefail -o errexit

# define colors
RESET='\033[0m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m]'

# Define arrays for services, etc.
# Read service arrays from environment variables or use default empty arrays
read -r -a payments <<< "${PAYMENTS[@]}"
read -r -a payouts <<< "${PAYOUTS[@]:-}"
read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST[@]:-}"
read -r -a routing <<< "${ROUTING[@]:-}"

connector_map=("payments" "payouts")

# Check if PAYMENT_METHOD_LIST is exported (even as empty)
if [[ -n "${PAYMENT_METHOD_LIST+x}" ]]; then
  connector_map+=("payment_method_list")
fi

# Check if ROUTING is exported (even as empty)
if [[ -n "${ROUTING+x}" ]]; then
  connector_map+=("routing")
fi

failed_connectors=()

# Function to check if a command exists
function command_exists() {
  local cmd="${1}"
  command -v "${cmd}" > /dev/null 2>&1
}

# Function to execute Cypress tests
function execute_test() {
  local connector="${1}"
  local service="${2}"
  echo "Executing tests for ${service} with connector ${connector}..."

  export REPORT_NAME="${service}_${connector}_report"
  if ! CYPRESS_CONNECTOR="$connector" npm run "cypress:$service"; then
    failed_connectors+=("${connector}-${service}")
  fi
}

export -f execute_test

# Function to run tests
function run_tests() {
  local jobs="${1:-1}"
  local tasks=()

  for service in "${connector_map[@]}"; do
    declare -n connectors="${service}"

    if [[ ${#connectors[@]} -eq 0 ]]; then
      # A service level test i.e., payment method list or routing
      [[ $service == "payment_method_list" ]] && service="payment-method-list"

      echo "${GREEN}Running ${service} tests without connectors...${RESET}"
      export REPORT_NAME="${service}_report"

      if ! npm run "cypress:${service}"; then
        failed_connectors+=("${service}")
      fi
    else
      # Connector test, i.e., payments or payouts
      echo -e "${GREEN}Running tests for service: '${service}'\nWith connectors: [${connectors[*]}] in batch of ${jobs}..${RESET}."
      echo "${connectors[@]}" | tr ' ' '\n' | parallel --jobs "${jobs}" execute_test {} "${service}"
    fi
  done

  if [ ${#failed_connectors[@]} -gt 0 ]; then
    echo -e "${RED}One or more connectors failed to run:${RESET}"
    printf '%s\n' "${failed_connectors[@]}"
    exit 1
  fi
}

function check_dependencies() {
  # parallel and npm are mandatory dependencies. exit the script if not found.
  # Check if gnu-parallel exist
  if ! command_exists 'parallel'; then
    echo "${RED}ERROR: GNU Parallel is not installed!${RESET}"
    exit 1
  fi

  # Check if npm is installed
  if ! command_exists 'npm'; then
    echo "${RED}ERROR: NPM is not installed!${RESET}"
    exit 1
  else
    # Re-install packages just so that they're intact
    npm ci
  fi
}

function main() {
  local command="${1:-}"
  local jobs="${2:-5}"

  if [[ $(basename "$(pwd)") != "cypress-tests" ]]; then
    echo "Changing directory to 'cypress-tests'..."
    cd cypress-tests || {
      echo "${RED}ERROR: Directory 'cypress-tests' not found!${RESET}"
      exit 1
    }
  fi

  check_dependencies

  case "${command}" in
    --parallel | -p)
      # At present, parallel execution is limited to batch of 5 to not run out of memory
      echo "${YELLOW}WARNING: Running Cypress tests in parallel is more resource intensive!${RESET}"
      run_tests "${jobs}"
      ;;
    *)
      run_tests 1
      ;;
  esac
}

# Execute the main function with passed arguments
main "$@"
