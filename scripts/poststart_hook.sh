#! /usr/bin/env bash
set -euo pipefail

# Configuration
VERSION="unknown"
STATUS=""
SERVER_BASE_URL="http://hyperswitch-server:8080"
HYPERSWITCH_HEALTH_URL="${SERVER_BASE_URL}/health"
HYPERSWITCH_DEEP_HEALTH_URL="${SERVER_BASE_URL}/health/ready"
WEBHOOK_URL="https://hyperswitch.gateway.scarf.sh/docker"

# Fetch health status
echo "Fetching app server health status..."
HEALTH_RESPONSE=$(curl --silent --fail "${HYPERSWITCH_HEALTH_URL}") || HEALTH_RESPONSE="connection_error"

if [[ "${HEALTH_RESPONSE}" == "connection_error" ]]; then
    STATUS="error"
    ERROR_MESSAGE="404 response"

    curl --get "${WEBHOOK_URL}" \
        --data-urlencode "version=${VERSION}" \
        --data-urlencode "status=${STATUS}" \
        --data-urlencode "error_message=${ERROR_MESSAGE}"

    echo "Webhook sent with connection error."
    exit 0
fi

# Fetch Hyperswitch version
VERSION=$(curl --silent --output /dev/null --request GET --write-out '%header{x-hyperswitch-version}' "${HYPERSWITCH_DEEP_HEALTH_URL}" | sed 's/-dirty$//')

echo "Fetching Hyperswitch health status..."
HEALTH_RESPONSE=$(curl --silent "${HYPERSWITCH_DEEP_HEALTH_URL}")

# Prepare curl command
CURL_COMMAND=("curl" "--get" "${WEBHOOK_URL}" "--data-urlencode" "version=${VERSION}")

# Check if the response contains an error
if [[ "$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error')" != 'null' ]]; then
    STATUS="error"
    ERROR_TYPE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.type')
    ERROR_MESSAGE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.message')
    ERROR_CODE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.code')

    CURL_COMMAND+=(
        "--data-urlencode" "status=${STATUS}"
        "--data-urlencode" "error_type='${ERROR_TYPE}'"
        "--data-urlencode" "error_message='${ERROR_MESSAGE}'"
        "--data-urlencode" "error_code='${ERROR_CODE}'"
    )
else
    STATUS="success"
    CURL_COMMAND+=("--data-urlencode" "status=${STATUS}")

    for key in $(echo "${HEALTH_RESPONSE}" | jq --raw-output 'keys_unsorted[]'); do
        value=$(echo "${HEALTH_RESPONSE}" | jq --raw-output --arg key "${key}" '.[$key]')
        CURL_COMMAND+=("--data-urlencode" "'${key}=${value}'")
    done
fi

# Send the webhook request
bash -c "${CURL_COMMAND[*]}"

echo "Webhook notification sent."
