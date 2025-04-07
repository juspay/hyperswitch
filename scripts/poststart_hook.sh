#! /usr/bin/env sh
set -euo pipefail

# Configuration
PLATFORM="docker"  # Change to "helm" or "cdk" as needed
VERSION="unknown"
STATUS=""
HYPERSWITCH_HEALTH_URL="http://hyperswitch-server:8080/health"
HYPERSWITCH_DEEP_HEALTH_URL="http://hyperswitch-server:8080/health/ready"
WEBHOOK_URL="https://hyperswitch.gateway.scarf.sh/${PLATFORM}"

# Fetch health status
echo "Fetching app server health status..."
HEALTH_RESPONSE=$(curl --silent --fail "${HYPERSWITCH_HEALTH_URL}") || HEALTH_RESPONSE="connection_error"

if [ "$HEALTH_RESPONSE" = "connection_error" ]; then
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
CURL_COMMAND="curl --get '${WEBHOOK_URL}' --data-urlencode 'version=${VERSION}'"

# Check if the response contains an error
if echo "${HEALTH_RESPONSE}" | grep -q '"error"'; then
    STATUS="error"
    ERROR_TYPE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.type')
    ERROR_MESSAGE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.message')
    ERROR_CODE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.code')

    CURL_COMMAND="${CURL_COMMAND} --data-urlencode 'status=${STATUS}'"
    CURL_COMMAND="${CURL_COMMAND} --data-urlencode 'error_type=${ERROR_TYPE}'"
    CURL_COMMAND="${CURL_COMMAND} --data-urlencode 'error_message=${ERROR_MESSAGE}'"
    CURL_COMMAND="${CURL_COMMAND} --data-urlencode 'error_code=${ERROR_CODE}'"
else
    STATUS="success"
    CURL_COMMAND="${CURL_COMMAND} --data-urlencode 'status=${STATUS}'"

    for key in $(echo "${HEALTH_RESPONSE}" | jq --raw-output 'keys_unsorted[]'); do
        value=$(echo "${HEALTH_RESPONSE}" | jq --raw-output --arg key "${key}" '.[$key]')
        CURL_COMMAND="${CURL_COMMAND} --data-urlencode '${key}=${value}'"
    done
fi

# Send the webhook request
sh -c "${CURL_COMMAND}"

echo "Webhook notification sent."
