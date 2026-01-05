# #!/usr/bin/env bash

# set -euo pipefail

# SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# # ----------------------------------
# # Cleanup on Ctrl+C or script exit
# # ----------------------------------
# cleanup() {
#   echo ""
#   echo "Caught interrupt. Cleaning up..."

#   # Kill server on port 8080 if running
#   if lsof -ti :8080 >/dev/null 2>&1; then
#     echo "Stopping server on port 8080..."
#     kill -9 $(lsof -ti :8080) 2>/dev/null || true
#   fi

#   exit 130
# }

# trap cleanup SIGINT SIGTERM

# # ----------------------------------
# # Read connector auth file path
# # ----------------------------------
# if [[ -z "${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-}" ]]; then
#   echo ""
#   read -r -p "Enter path to connector creds file (e.g. /Downloads/creds.json): " USER_CREDS_PATH

#   # Expand ~ to home directory
#   USER_CREDS_PATH="${USER_CREDS_PATH/#\~/$HOME}"

#   if [[ ! -f "$USER_CREDS_PATH" ]]; then
#     echo "ERROR: File not found: $USER_CREDS_PATH"
#     exit 1
#   fi

#   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$USER_CREDS_PATH"
# fi

# echo "Using creds file: $CYPRESS_CONNECTOR_AUTH_FILE_PATH"


# # ----------------------------------
# # Input validation
# # ----------------------------------
# if [ "$#" -eq 0 ]; then
#   echo "Usage: $0 connector1 connector2 connector3 ..."
#   exit 1
# fi

# CONNECTORS=("$@")

# # ----------------------------------
# # Cypress prerequisites (GLOBAL)
# # ----------------------------------
# export CYPRESS_ADMINAPIKEY="test_admin"
# export CYPRESS_BASEURL="http://localhost:8080"
# # export CYPRESS_CONNECTOR_AUTH_FILE_PATH="/Users/nishanth.challa/Downloads/creds.json"

# echo "Cypress prerequisites exported."

# # ----------------------------------
# # Check if port 8080 is running
# # ----------------------------------
# if ! lsof -i :8080 >/dev/null 2>&1; then
#   echo "Port 8080 not in use. Starting server using 'cargo run'..."
#   cargo run &
# fi

# # ----------------------------------
# # Wait for server to come up
# # ----------------------------------
# echo "Waiting for server on port 8080..."
# until lsof -i :8080 >/dev/null 2>&1; do
#   sleep 1
# done
# echo "Server is up on port 8080."

# # ----------------------------------
# # Ensure Homebrew bash exists
# # ----------------------------------
# BASH_BIN="/opt/homebrew/bin/bash"

# if [ ! -x "$BASH_BIN" ]; then
#   echo "Homebrew bash not found. Installing..."
#   brew install bash
# fi

# # ----------------------------------
# # Detect CPU cores
# # ----------------------------------
# CORES=$(sysctl -n hw.ncpu)

# if [ "$CORES" -gt 10 ]; then
#   BATCH_SIZE=5
# else
#   BATCH_SIZE=3
# fi

# echo "Detected $CORES cores → batch size = $BATCH_SIZE"

# # ----------------------------------
# # Run connectors in batches
# # ----------------------------------
# TOTAL=${#CONNECTORS[@]}
# INDEX=0

# while [ "$INDEX" -lt "$TOTAL" ]; do
#   CURRENT_BATCH=("${CONNECTORS[@]:$INDEX:$BATCH_SIZE}")
#   CURRENT_COUNT=${#CURRENT_BATCH[@]}

#   CONNECTOR_LIST=$(IFS=,; echo "${CURRENT_BATCH[*]}")

#   export PAYMENTS_CONNECTORS="$CONNECTOR_LIST"

#   echo "Running batch ($CURRENT_COUNT): $PAYMENTS_CONNECTORS"

# (
#   cd "$REPO_ROOT"
#   "$BASH_BIN" "$SCRIPT_DIR/execute_cypress.sh" --parallel "$CURRENT_COUNT"
# )

#   INDEX=$((INDEX + BATCH_SIZE))
# done

# echo "All connector batches executed successfully."



# #!/usr/bin/env bash
# set -euo pipefail

# # --------------------------------------------------
# # Resolve paths
# # --------------------------------------------------
# SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# # --------------------------------------------------
# # Globals for server lifecycle
# # --------------------------------------------------
# SERVER_PID=""
# SERVER_STARTED_BY_SCRIPT=false

# # --------------------------------------------------
# # Cleanup handler
# # --------------------------------------------------
# cleanup() {
#   echo ""
#   echo "Cleaning up..."

#   if [[ "$SERVER_STARTED_BY_SCRIPT" == true ]] && [[ -n "${SERVER_PID:-}" ]]; then
#     if kill -0 "$SERVER_PID" 2>/dev/null; then
#       echo "Stopping server started by this script (PID $SERVER_PID)..."
#       kill -9 "$SERVER_PID" || true
#     fi
#   else
#     echo "Server was not started by this script. Leaving it running."
#   fi

#   exit 130
# }

# trap cleanup SIGINT SIGTERM EXIT

# # --------------------------------------------------
# # Input validation
# # --------------------------------------------------
# if [[ "$#" -eq 0 ]]; then
#   echo "Usage: $0 connector1 connector2 ..."
#   exit 1
# fi

# CONNECTORS=("$@")

# # --------------------------------------------------
# # Cypress prerequisites
# # --------------------------------------------------
# export CYPRESS_ADMINAPIKEY="test_admin"
# export CYPRESS_BASEURL="http://localhost:8080"

# # --------------------------------------------------
# # Ask user for creds file path if not set
# # --------------------------------------------------
# if [[ -z "${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-}" ]]; then
#   echo ""
#   read -r -p "Enter path to connector creds file (e.g. ~/Downloads/creds.json): " USER_CREDS_PATH

#   USER_CREDS_PATH="${USER_CREDS_PATH/#\~/$HOME}"

#   if [[ ! -f "$USER_CREDS_PATH" ]]; then
#     echo "ERROR: File not found: $USER_CREDS_PATH"
#     exit 1
#   fi

#   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$USER_CREDS_PATH"
# fi

# echo "Using creds file: $CYPRESS_CONNECTOR_AUTH_FILE_PATH"
# echo "Cypress prerequisites exported."

# # --------------------------------------------------
# # Start server only if needed
# # --------------------------------------------------
# if lsof -ti :8080 >/dev/null 2>&1; then
#   echo "Server already running on port 8080. Will not manage its lifecycle."
# else
#   echo "Port 8080 not in use. Starting server..."
#   cargo run &
#   SERVER_PID=$!
#   SERVER_STARTED_BY_SCRIPT=true
# fi

# # --------------------------------------------------
# # Wait for server to be ready
# # --------------------------------------------------
# echo "Waiting for server on port 8080..."
# until lsof -ti :8080 >/dev/null 2>&1; do
#   sleep 1
# done
# echo "Server is up on port 8080."

# # --------------------------------------------------
# # Ensure Homebrew bash exists
# # --------------------------------------------------
# BASH_BIN="/opt/homebrew/bin/bash"

# if [[ ! -x "$BASH_BIN" ]]; then
#   echo "Homebrew bash not found. Installing..."
#   brew install bash
# fi

# # --------------------------------------------------
# # Detect CPU cores → batch size
# # --------------------------------------------------
# CORES=$(sysctl -n hw.ncpu)

# if [[ "$CORES" -gt 10 ]]; then
#   BATCH_SIZE=2
# else
#   BATCH_SIZE=1
# fi

# echo "Detected $CORES cores → batch size = $BATCH_SIZE"

# # --------------------------------------------------
# # Run connectors in batches
# # --------------------------------------------------
# TOTAL=${#CONNECTORS[@]}
# INDEX=0

# while [[ "$INDEX" -lt "$TOTAL" ]]; do
#   CURRENT_BATCH=("${CONNECTORS[@]:$INDEX:$BATCH_SIZE}")
#   CURRENT_COUNT=${#CURRENT_BATCH[@]}
#   CONNECTOR_LIST="${CURRENT_BATCH[*]}"

#   export PAYMENTS_CONNECTORS="$CONNECTOR_LIST"

#   echo "Running batch ($CURRENT_COUNT): $PAYMENTS_CONNECTORS"

#   (
#     cd "$REPO_ROOT"
#     "$BASH_BIN" "$SCRIPT_DIR/execute_cypress.sh" --parallel "$CURRENT_COUNT"
#   )

#   INDEX=$((INDEX + BATCH_SIZE))
# done

# echo "All connector batches executed successfully."


# #!/usr/bin/env bash
# set -euo pipefail

# # --------------------------------------------------
# # Resolve paths
# # --------------------------------------------------
# SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# # --------------------------------------------------
# # Globals
# # --------------------------------------------------
# SERVER_PID=""
# SERVER_STARTED_BY_SCRIPT=false
# ANY_BATCH_FAILED=false

# # --------------------------------------------------
# # Cleanup (no exit here)
# # --------------------------------------------------
# cleanup() {
#   echo ""
#   echo "Cleaning up..."

#   if [[ "$SERVER_STARTED_BY_SCRIPT" == true ]] && [[ -n "${SERVER_PID:-}" ]]; then
#     if kill -0 "$SERVER_PID" 2>/dev/null; then
#       echo "Stopping server started by this script (PID $SERVER_PID)..."
#       kill -9 "$SERVER_PID" || true
#     fi
#   else
#     echo "Server was not started by this script. Leaving it running."
#   fi
# }

# # --------------------------------------------------
# # Ctrl+C / SIGTERM handler
# # --------------------------------------------------
# on_interrupt() {
#   echo ""
#   echo "Interrupted by user."
#   cleanup
#   exit 130
# }

# trap on_interrupt SIGINT SIGTERM

# # --------------------------------------------------
# # Input validation
# # --------------------------------------------------
# if [[ "$#" -eq 0 ]]; then
#   echo "Usage: $0 connector1 connector2 ..."
#   exit 1
# fi

# CONNECTORS=("$@")

# # --------------------------------------------------
# # Cypress prerequisites
# # --------------------------------------------------
# export CYPRESS_ADMINAPIKEY="test_admin"
# export CYPRESS_BASEURL="http://localhost:8080"

# # --------------------------------------------------
# # Ask for creds file if not set
# # --------------------------------------------------
# if [[ -z "${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-}" ]]; then
#   echo ""
#   read -r -p "Enter path to connector creds file (e.g. ~/Downloads/creds.json): " USER_CREDS_PATH
#   USER_CREDS_PATH="${USER_CREDS_PATH/#\~/$HOME}"

#   if [[ ! -f "$USER_CREDS_PATH" ]]; then
#     echo "ERROR: File not found: $USER_CREDS_PATH"
#     exit 1
#   fi

#   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$USER_CREDS_PATH"
# fi

# echo "Using creds file: $CYPRESS_CONNECTOR_AUTH_FILE_PATH"
# echo "Cypress prerequisites exported."

# # --------------------------------------------------
# # Start server if needed
# # --------------------------------------------------
# if lsof -ti :8080 >/dev/null 2>&1; then
#   echo "Server already running on port 8080. Will not manage its lifecycle."
# else
#   echo "Port 8080 not in use. Starting server..."
#   cargo run &
#   SERVER_PID=$!
#   SERVER_STARTED_BY_SCRIPT=true
# fi

# # --------------------------------------------------
# # Wait for server
# # --------------------------------------------------
# echo "Waiting for server on port 8080..."
# until lsof -ti :8080 >/dev/null 2>&1; do
#   sleep 1
# done
# echo "Server is up on port 8080."

# # --------------------------------------------------
# # Ensure Homebrew bash
# # --------------------------------------------------
# BASH_BIN="/opt/homebrew/bin/bash"
# if [[ ! -x "$BASH_BIN" ]]; then
#   echo "Homebrew bash not found. Installing..."
#   brew install bash
# fi

# # --------------------------------------------------
# # Compute batch size
# # --------------------------------------------------
# CORES=$(sysctl -n hw.ncpu)
# if [[ "$CORES" -gt 10 ]]; then
#   BATCH_SIZE=2
# else
#   BATCH_SIZE=1
# fi

# echo "Detected $CORES cores → batch size = $BATCH_SIZE"

# # --------------------------------------------------
# # Run batches (THIS IS THE FIXED PART)
# # --------------------------------------------------
# TOTAL=${#CONNECTORS[@]}
# INDEX=0

# while [[ "$INDEX" -lt "$TOTAL" ]]; do
#   CURRENT_BATCH=("${CONNECTORS[@]:$INDEX:$BATCH_SIZE}")
#   export PAYMENTS_CONNECTORS="${CURRENT_BATCH[*]}"

#   echo "Running batch (${#CURRENT_BATCH[@]}): $PAYMENTS_CONNECTORS"

#   if ! (
#     cd "$REPO_ROOT"
#     "$BASH_BIN" "$SCRIPT_DIR/execute_cypress.sh" --parallel "$BATCH_SIZE"
#   ); then
#     ANY_BATCH_FAILED=true
#   fi

#   INDEX=$((INDEX + BATCH_SIZE))
# done

# # --------------------------------------------------
# # Final result + cleanup
# # --------------------------------------------------
# if [[ "$ANY_BATCH_FAILED" == true ]]; then
#   echo ""
#   echo "❌ One or more batches had failing connectors."
#   cleanup
#   exit 1
# else
#   echo ""
#   echo "✅ All connector batches executed successfully."
#   cleanup
#   exit 0
# fi


# #!/usr/bin/env bash
# set -euo pipefail

# # ==================================================
# # Resolve paths
# # ==================================================
# SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# # ==================================================
# # Globals
# # ==================================================
# SERVER_PID=""
# SERVER_STARTED_BY_SCRIPT=false
# ANY_BATCH_FAILED=false
# TMP_CREDS_FILE=""

# # ==================================================
# # DB config
# # ==================================================
# DB_HOST="localhost"
# DB_PORT="5432"
# DB_NAME="hyperswitch_db"
# DB_USER="db_user"
# CARDS_CSV="$REPO_ROOT/.github/data/cards_info.csv"

# # ==================================================
# # GitHub / internal config
# # ==================================================
# GITHUB_ORG="juspay"
# INTERNAL_ENC_CREDS="$REPO_ROOT/.secure/creds.enc"

# # ==================================================
# # Utility helpers
# # ==================================================
# ask_yes_no() {
#   local ans
#   while true; do
#     read -r -p "$1 (yes/no): " ans
#     case "$ans" in
#       yes|y) echo "yes"; return ;;
#       no|n)  echo "no"; return ;;
#       *) echo "Please answer yes or no." ;;
#     esac
#   done
# }

# # ==================================================
# # DB seed
# # ==================================================
# seed_db() {
#   echo ""
#   echo "Seeding cards_info table..."

#   set +e
#   psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <<EOF
# BEGIN;
# TRUNCATE TABLE cards_info;
# \copy cards_info FROM '$CARDS_CSV' DELIMITER ',' CSV HEADER;
# COMMIT;
# EOF
#   set -e

#   echo "DB seed attempted (non-fatal)."
# }

# # ==================================================
# # DB cleanup
# # ==================================================
# cleanup_db() {
#   echo ""
#   echo "Cleaning cards_info table..."

#   set +e
#   psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
#     -c "TRUNCATE TABLE cards_info;" >/dev/null 2>&1
#   set -e
# }

# # ==================================================
# # Cleanup (ALWAYS runs)
# # ==================================================
# cleanup() {
#   echo ""
#   echo "Running cleanup..."

#   cleanup_db

#   if [[ -n "${TMP_CREDS_FILE:-}" && -f "$TMP_CREDS_FILE" ]]; then
#     shred -u "$TMP_CREDS_FILE"
#     echo "Decrypted creds wiped."
#   fi

#   if [[ "$SERVER_STARTED_BY_SCRIPT" == true ]] && [[ -n "${SERVER_PID:-}" ]]; then
#     if kill -0 "$SERVER_PID" 2>/dev/null; then
#       echo "Stopping server (PID $SERVER_PID)..."
#       kill -9 "$SERVER_PID" || true
#     fi
#   else
#     echo "Server not started by this script."
#   fi
# }

# trap cleanup EXIT
# trap 'echo "Interrupted"; exit 130' SIGINT SIGTERM

# # ==================================================
# # Input validation
# # ==================================================
# if [[ "$#" -eq 0 ]]; then
#   echo "Usage: $0 connector1 connector2 ..."
#   exit 1
# fi

# CONNECTORS=("$@")

# # ==================================================
# # Identify user & resolve creds
# # ==================================================
# IS_JUSPAY=$(ask_yes_no "Are you a Juspay employee")

# if [[ "$IS_JUSPAY" == "yes" ]]; then
#   echo ""
#   read -r -p "GitHub username: " GH_USER
#   read -r -s -p "GitHub token (org:read): " GH_TOKEN
#   echo ""

#   echo "Verifying GitHub org membership..."

#   STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
#     -H "Accept: application/vnd.github+json" \
#     -H "Authorization: Bearer $GH_TOKEN" \
#     -H "X-GitHub-Api-Version: 2022-11-28" \
#     "https://api.github.com/orgs/$GITHUB_ORG/public_members/$GH_USER")

#   if [[ "$STATUS" != "204" ]]; then
#     echo "❌ Not a public member of $GITHUB_ORG."
#     exit 1
#   fi

#   echo "✅ Verified Juspay member."

#   if [[ ! -f "$INTERNAL_ENC_CREDS" ]]; then
#     echo "❌ Encrypted creds file missing."
#     exit 1
#   fi

#   read -r -s -p "Enter decryption key: " DECRYPT_KEY
#   echo ""

#   TMP_CREDS_FILE="$(mktemp)"
#   chmod 600 "$TMP_CREDS_FILE"

#   openssl enc -d -aes-256-cbc -pbkdf2 \
#     -in "$INTERNAL_ENC_CREDS" \
#     -out "$TMP_CREDS_FILE" \
#     -pass pass:"$DECRYPT_KEY" || {
#       echo "❌ Decryption failed."
#       exit 1
#     }

#   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$TMP_CREDS_FILE"
#   echo "Internal creds loaded securely."

# else
#   echo ""
#   read -r -p "Path to creds.json: " USER_CREDS
#   USER_CREDS="${USER_CREDS/#\~/$HOME}"

#   if [[ ! -f "$USER_CREDS" ]]; then
#     echo "❌ Creds file not found."
#     exit 1
#   fi

#   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$USER_CREDS"
#   echo "Using external creds."
# fi

# # ==================================================
# # Cypress prerequisites
# # ==================================================
# export CYPRESS_ADMINAPIKEY="test_admin"
# export CYPRESS_BASEURL="http://localhost:8080"

# # ==================================================
# # Seed DB (always)
# # ==================================================
# seed_db

# # ==================================================
# # Start server if needed
# # ==================================================
# if lsof -ti :8080 >/dev/null 2>&1; then
#   echo "Server already running."
# else
#   echo "Starting Hyperswitch..."
#   (cd "$REPO_ROOT" && cargo run) &
#   SERVER_PID=$!
#   SERVER_STARTED_BY_SCRIPT=true
# fi

# echo "Waiting for server..."
# until lsof -ti :8080 >/dev/null 2>&1; do sleep 1; done
# echo "Server ready."

# # ==================================================
# # Ensure Homebrew bash
# # ==================================================
# BASH_BIN="/opt/homebrew/bin/bash"
# if [[ ! -x "$BASH_BIN" ]]; then
#   brew install bash
# fi

# # ==================================================
# # Compute batch size
# # ==================================================
# CORES=$(sysctl -n hw.ncpu)
# BATCH_SIZE=1
# [[ "$CORES" -ge 8 ]] && BATCH_SIZE=2

# echo "Batch size: $BATCH_SIZE"

# # ==================================================
# # Run Cypress batches
# # ==================================================
# TOTAL=${#CONNECTORS[@]}
# INDEX=0

# while [[ "$INDEX" -lt "$TOTAL" ]]; do
#   CURRENT_BATCH=("${CONNECTORS[@]:$INDEX:$BATCH_SIZE}")
#   export PAYMENTS_CONNECTORS="${CURRENT_BATCH[*]}"

#   echo ""
#   echo "▶ Running batch: $PAYMENTS_CONNECTORS"

#   if ! (
#     set +e
#     cd "$REPO_ROOT"
#     "$BASH_BIN" "$SCRIPT_DIR/execute_cypress.sh" --parallel "${#CURRENT_BATCH[@]}"
#   ); then
#     ANY_BATCH_FAILED=true
#     echo "❌ Batch failed."
#   else
#     echo "✅ Batch passed."
#   fi

#   INDEX=$((INDEX + BATCH_SIZE))
# done

# # ==================================================
# # Final status
# # ==================================================
# if [[ "$ANY_BATCH_FAILED" == true ]]; then
#   echo ""
#   echo "❌ One or more batches failed."
#   exit 1
# else
#   echo ""
#   echo "✅ All batches completed successfully."
#   exit 0
# fi


#!/usr/bin/env bash
set -euo pipefail

# ==================================================
# Resolve paths
# ==================================================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ==================================================
# Globals
# ==================================================
SERVER_PID=""
SERVER_STARTED_BY_SCRIPT=false
ANY_BATCH_FAILED=false
TMP_CREDS_FILE=""

declare -A CONNECTOR_PASSED
declare -A CONNECTOR_FAILED
declare -A CONNECTOR_FAILED_TESTS

# ==================================================
# DB config
# ==================================================
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="hyperswitch_db"
DB_USER="db_user"
CARDS_CSV="$REPO_ROOT/.github/data/cards_info.csv"

# ==================================================
# GitHub / internal config
# ==================================================
GITHUB_ORG="juspay"
INTERNAL_ENC_CREDS="$REPO_ROOT/.secure/creds.enc"

# ==================================================
# Utility helpers
# ==================================================
ask_yes_no() {
  local ans
  while true; do
    read -r -p "$1 (yes/no): " ans
    case "$ans" in
      yes|y) echo "yes"; return ;;
      no|n)  echo "no"; return ;;
      *) echo "Please answer yes or no." ;;
    esac
  done
}

# ==================================================
# DB seed
# ==================================================
seed_db() {
  echo ""
  echo "Seeding cards_info table..."

  set +e
  psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <<EOF
BEGIN;
TRUNCATE TABLE cards_info;
\copy cards_info FROM '$CARDS_CSV' DELIMITER ',' CSV HEADER;
COMMIT;
EOF
  set -e

  echo "DB seed attempted (non-fatal)."
}

# ==================================================
# DB cleanup
# ==================================================
cleanup_db() {
  echo ""
  echo "Cleaning cards_info table..."

  set +e
  psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
    -c "TRUNCATE TABLE cards_info;" >/dev/null 2>&1
  set -e
}

# ==================================================
# Cleanup (ALWAYS runs)
# ==================================================
cleanup() {
  echo ""
  echo "Running cleanup..."

  cleanup_db

  if [[ -n "${TMP_CREDS_FILE:-}" && -f "$TMP_CREDS_FILE" ]]; then
    shred -u "$TMP_CREDS_FILE"
    echo "Decrypted creds wiped."
  fi

  if [[ "$SERVER_STARTED_BY_SCRIPT" == true ]] && [[ -n "${SERVER_PID:-}" ]]; then
    if kill -0 "$SERVER_PID" 2>/dev/null; then
      echo "Stopping server (PID $SERVER_PID)..."
      kill -9 "$SERVER_PID" || true
    fi
  else
    echo "Server not started by this script."
  fi
}

trap cleanup EXIT
trap 'echo "Interrupted"; exit 130' SIGINT SIGTERM

# ==================================================
# Input validation
# ==================================================
if [[ "$#" -eq 0 ]]; then
  echo "Usage: $0 connector1 connector2 ..."
  exit 1
fi

CONNECTORS=("$@")

# ==================================================
# Identify user & resolve creds
# ==================================================
IS_JUSPAY=$(ask_yes_no "Are you a Juspay employee")

if [[ "$IS_JUSPAY" == "yes" ]]; then
  echo ""
  read -r -p "GitHub username: " GH_USER
  read -r -s -p "GitHub token (org:read): " GH_TOKEN
  echo ""

  echo "Verifying GitHub org membership..."

  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $GH_TOKEN" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "https://api.github.com/orgs/$GITHUB_ORG/public_members/$GH_USER")

  if [[ "$STATUS" != "204" ]]; then
    echo "❌ Not a public member of $GITHUB_ORG."
    exit 1
  fi

  echo "✅ Verified Juspay member."

  read -r -s -p "Enter decryption key: " DECRYPT_KEY
  echo ""

  TMP_CREDS_FILE="$(mktemp)"
  chmod 600 "$TMP_CREDS_FILE"

  openssl enc -d -aes-256-cbc -pbkdf2 \
    -in "$INTERNAL_ENC_CREDS" \
    -out "$TMP_CREDS_FILE" \
    -pass pass:"$DECRYPT_KEY" || {
      echo "❌ Decryption failed."
      exit 1
    }

  export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$TMP_CREDS_FILE"
  echo "Internal creds loaded securely."

else
  echo ""
  read -r -p "Path to creds.json: " USER_CREDS
  USER_CREDS="${USER_CREDS/#\~/$HOME}"

  [[ ! -f "$USER_CREDS" ]] && { echo "❌ Creds file not found."; exit 1; }

  export CYPRESS_CONNECTOR_AUTH_FILE_PATH="$USER_CREDS"
  echo "Using external creds."
fi

# ==================================================
# Cypress prerequisites
# ==================================================
export CYPRESS_ADMINAPIKEY="test_admin"
export CYPRESS_BASEURL="http://localhost:8080"

# ==================================================
# Seed DB
# ==================================================
seed_db

# ==================================================
# Start Hyperswitch server
# ==================================================
if ! lsof -ti :8080 >/dev/null 2>&1; then
  echo "Starting Hyperswitch..."
  (cd "$REPO_ROOT" && cargo run) &
  SERVER_PID=$!
  SERVER_STARTED_BY_SCRIPT=true
fi

echo "Waiting for server..."
until lsof -ti :8080 >/dev/null 2>&1; do sleep 1; done
echo "Server ready."

# ==================================================
# Ensure bash
# ==================================================
BASH_BIN="/opt/homebrew/bin/bash"
[[ ! -x "$BASH_BIN" ]] && brew install bash

# ==================================================
# Compute batch size
# ==================================================
CORES=$(sysctl -n hw.ncpu)
BATCH_SIZE=3
[[ "$CORES" -ge 8 ]] && BATCH_SIZE=4

# ==================================================
# Report parser
# ==================================================
parse_report() {
  local connector="$1"
  local report_dir="$REPO_ROOT/cypress/reports"
  local json_file

  json_file=$(ls -t "$report_dir"/*.json 2>/dev/null | head -n 1)
  [[ -z "$json_file" ]] && return

  CONNECTOR_PASSED["$connector"]=$(jq '[.. | objects | select(.state=="passed")] | length' "$json_file")
  CONNECTOR_FAILED["$connector"]=$(jq '[.. | objects | select(.state=="failed")] | length' "$json_file")

  CONNECTOR_FAILED_TESTS["$connector"]=$(
    jq -r '.. | objects | select(.state=="failed") | .fullTitle // empty' "$json_file"
  )
}

# ==================================================
# Run Cypress batches
# ==================================================
TOTAL=${#CONNECTORS[@]}
INDEX=0

while [[ "$INDEX" -lt "$TOTAL" ]]; do
  CURRENT_BATCH=("${CONNECTORS[@]:$INDEX:$BATCH_SIZE}")
  export PAYMENTS_CONNECTORS="${CURRENT_BATCH[*]}"

  echo ""
  echo "▶ Running batch: $PAYMENTS_CONNECTORS"

  if ! (
    set +e
    cd "$REPO_ROOT"
    "$BASH_BIN" "$SCRIPT_DIR/execute_cypress.sh" --parallel "${#CURRENT_BATCH[@]}"
  ); then
    ANY_BATCH_FAILED=true
  fi

  for c in "${CURRENT_BATCH[@]}"; do
    parse_report "$c"
  done

  INDEX=$((INDEX + BATCH_SIZE))
done

# ==================================================
# Final report
# ==================================================
echo ""
echo "================ Connector Test Report ================"

for c in "${CONNECTORS[@]}"; do
  echo ""
  echo "Connector: $c"
  echo "  Passed: ${CONNECTOR_PASSED[$c]:-0}"
  echo "  Failed: ${CONNECTOR_FAILED[$c]:-0}"

  if [[ "${CONNECTOR_FAILED[$c]:-0}" -gt 0 ]]; then
    echo "  Failed Tests:"
    while read -r t; do
      [[ -n "$t" ]] && echo "    - $t"
    done <<< "${CONNECTOR_FAILED_TESTS[$c]}"
  fi
done

echo "======================================================"

# ==================================================
# JSON report
# ==================================================
REPORT_FILE="$REPO_ROOT/cypress/reports/connector-summary.json"

{
  echo "{"
  for c in "${CONNECTORS[@]}"; do
    echo "  \"$c\": {"
    echo "    \"passed\": ${CONNECTOR_PASSED[$c]:-0},"
    echo "    \"failed\": ${CONNECTOR_FAILED[$c]:-0}"
    echo "  },"
  done | sed '$ s/,$//'
  echo "}"
} > "$REPORT_FILE"

echo "📄 Summary written to $REPORT_FILE"

# ==================================================
# Exit status
# ==================================================
[[ "$ANY_BATCH_FAILED" == true ]] && exit 1 || exit 0

