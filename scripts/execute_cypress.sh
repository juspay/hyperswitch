#!/usr/bin/env bash
set -euo pipefail

#! initialinzing temp file
tmp_file=""

cleanup() {
  print_color yellow "Cleaning up..."
  if [[ -n "${tmp_file:-}" && -f "$tmp_file" ]]; then
    rm -f "$tmp_file"
  fi
}

trap cleanup EXIT

# -----------------------------
# Color logging
# -----------------------------
print_color() {
  local color="$1"; shift
  local reset='\033[0m'
  local red='\033[0;31m'
  local green='\033[0;32m'
  local yellow='\033[0;33m'
  echo -e "${!color}$*${reset}"
}
export -f print_color

# -----------------------------
# Dependency check
# -----------------------------
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

check_dependencies() {
  local deps=("parallel" "npm")

  for dep in "${deps[@]}"; do
    if ! command_exists "$dep"; then
      print_color red "ERROR: ${dep} is not installed"
      exit 1
    fi
  done

  print_color green "Dependencies OK"
  npm ci
}

# -----------------------------
# Read env → arrays
# -----------------------------
read -r -a payments <<< "${PAYMENTS_CONNECTORS:-}"
read -r -a payouts <<< "${PAYOUTS_CONNECTORS:-}"
read -r -a payment_method_list <<< "${PAYMENT_METHOD_LIST:-}"
read -r -a routing <<< "${ROUTING:-}"

declare -A services=(
  ["PAYMENTS_CONNECTORS"]="payments"
  ["PAYOUTS_CONNECTORS"]="payouts"
  ["PAYMENT_METHOD_LIST"]="payment_method_list"
  ["ROUTING"]="routing"
)

connector_map=()

read_service_arrays() {
  for var in "${!services[@]}"; do
    if [[ -n "${!var:-}" ]]; then
      connector_map+=("${services[$var]}")
    else
      print_color yellow "Skipping ${var} (not set)"
    fi
  done
}

# -----------------------------
# Execute one Cypress job
# -----------------------------
execute_test() {
  local connector="$1"
  local service="$2"
  local tmp_file="$3"

  local start_ts end_ts duration
  start_ts=$(date +%s)

  print_color yellow \
    "[${service}:${connector}] START (PID=$$, at $(date '+%H:%M:%S'))"

  export CYPRESS_CONNECTOR="$connector"
  export REPORT_NAME="${service}_${connector}_report"

  # --- run Cypress, capture exit code safely ---
  local exit_code=0
  npm run "cypress:${service}" \
    2>&1 | sed "s/^/[${service}:${connector}] /" || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    echo "${service}-${connector}" >> "$tmp_file"
  fi

  end_ts=$(date +%s)
  duration=$(( end_ts - start_ts ))

  if [[ $exit_code -ne 0 ]]; then
    print_color red \
      "[${service}:${connector}] FAILED (PID=$$) took ${duration}s"
  else
    print_color green \
      "[${service}:${connector}] PASSED (PID=$$) took ${duration}s"
  fi

  return 0   
}
export -f execute_test


# -----------------------------
# Run all tests
# -----------------------------
run_tests() {
  local jobs="${1:-1}"
  tmp_file=$(mktemp)

  for service in "${connector_map[@]}"; do
    declare -n connectors="$service"

    if (( ${#connectors[@]} > 0 )); then
      parallel --jobs "$jobs" --group \
        execute_test ::: "${connectors[@]}" ::: "$service" ::: "$tmp_file" || true
    else
      if ! npm run "cypress:${service}"; then
        echo "$service" >> "$tmp_file"
      fi
    fi
  done

  # ✅ FINAL DECISION POINT
  if [[ -s "$tmp_file" ]]; then
    echo "❌ The following connectors failed:"
    sort -u "$tmp_file" | sed 's/^/  - /'
    rm -f "$tmp_file"
    exit 1
  else
    echo "✅ All connectors passed"
    rm -f "$tmp_file"
    exit 0
  fi
}



# -----------------------------
# Main
# -----------------------------
main() {
  local command="${1:-}"
  local jobs="${2:-3}"
  local test_dir="${3:-cypress-tests}"

  if [[ "$(basename "$PWD")" != "$(basename "$test_dir")" ]]; then
    print_color yellow "Changing directory to ${test_dir}"
    cd "$test_dir"
  fi

  check_dependencies
  read_service_arrays

  case "$command" in
    --parallel|-p)
      print_color yellow "Running tests in parallel (jobs=${jobs})"
      run_tests "$jobs"
      ;;
    *)
      print_color yellow "Running tests sequentially"
      run_tests 1
      ;;
  esac
}

main "$@"
