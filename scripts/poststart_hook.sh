#! /usr/bin/env bash
set -euo pipefail

# Configuration
VERSION="unknown"
STATUS=""
SERVER_BASE_URL="http://hyperswitch-server:8080"
HYPERSWITCH_HEALTH_URL="${SERVER_BASE_URL}/health"
HYPERSWITCH_DEEP_HEALTH_URL="${SERVER_BASE_URL}/health/ready"
ONE_CLICK_SETUP="${ONE_CLICK_SETUP:-false}"

if [[ "${ONE_CLICK_SETUP}" == "true" ]]; then
    SCARF_URL="https://hyperswitch.gateway.scarf.sh/docker"
else
    SCARF_URL="https://hyperswitch.gateway.scarf.sh/only-docker"
fi

# Fetch health status
echo "Fetching app server health status..."
HEALTH_RESPONSE=$(curl --silent --fail "${HYPERSWITCH_HEALTH_URL}") || HEALTH_RESPONSE="connection_error"

if [[ "${HEALTH_RESPONSE}" == "connection_error" ]]; then
    STATUS="error"
    ERROR_MESSAGE="500 response"

    curl --get "${SCARF_URL}" \
        --data-urlencode "version=${VERSION}" \
        --data-urlencode "status=${STATUS}" \
        --data-urlencode "error_message='${ERROR_MESSAGE}'"

    echo "Webhook sent with connection error."
    exit 0
fi

# Fetch Hyperswitch version
VERSION=$(curl --silent --output /dev/null --request GET --write-out '%header{x-hyperswitch-version}' "${HYPERSWITCH_DEEP_HEALTH_URL}" | sed 's/-dirty$//')

echo "Fetching Hyperswitch health status..."
HEALTH_RESPONSE=$(curl --silent "${HYPERSWITCH_DEEP_HEALTH_URL}")

# Prepare curl command
CURL_COMMAND=("curl" "--get" "${SCARF_URL}" "--data-urlencode" "version=${VERSION}")

# Check if the response contains an error
if [[ "$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error')" != "null" ]]; then
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
    "${CURL_COMMAND[@]}"
    echo "Webhook sent with error status."
    exit 0
elif [[ "${ONE_CLICK_SETUP}" == "false" ]]; then
    STATUS="success"
    CURL_COMMAND+=("--data-urlencode" "status=${STATUS}")

    for key in $(echo "${HEALTH_RESPONSE}" | jq --raw-output 'keys_unsorted[]'); do
        value=$(echo "${HEALTH_RESPONSE}" | jq --raw-output --arg key "${key}" '.[$key]')
        CURL_COMMAND+=("--data-urlencode" "${key}=${value}")
    done
    "${CURL_COMMAND[@]}"
    echo "Webhook notification sent for success status."
else
    echo "ONE_CLICK_SETUP=true and status=success, skipping webhook call."
fi
