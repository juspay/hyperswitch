#!/bin/sh

# Configuration
PLATFORM="docker"  # Change to "helm" or "cdk" as needed
VERSION="unknown"
STATUS=""
SERVER_HEALTH_URL="http://hyperswitch-server:8080/health"
HYPERSWITCH_URL="http://hyperswitch-server:8080/health/ready"
WEBHOOK_URL="https://hyperswitch.gateway.scarf.sh/$PLATFORM"

# Fetch health status
echo "Fetching app server health status..."
HEALTH_RESPONSE=$(curl -s --fail "$SERVER_HEALTH_URL") || HEALTH_RESPONSE="connection_error"

if [ "$HEALTH_RESPONSE" = "connection_error" ]; then
    STATUS="error"
    ERROR_MESSAGE=$(echo "404 response" | jq -sRr @uri)
    PARAMS="$PARAMS&status=$STATUS"
    PARAMS="$PARAMS&error_message=$ERROR_MESSAGE"
    curl -G "$WEBHOOK_URL?$PARAMS"
    exit 0
fi

#fetch hyperswitch version
VERSION=$(curl --silent --output /dev/null --request GET --write-out '%header{x-hyperswitch-version}' "$HYPERSWITCH_URL" | sed 's/-.*//' | jq -sRr @uri)

echo "Fetching Hyperswitch health status..."
HEALTH_RESPONSE=$(curl -s "$HYPERSWITCH_URL")

# Initialize parameters
PARAMS="version=$VERSION"

# Check if the response contains an error
if echo "$HEALTH_RESPONSE" | grep -q '"error"'; then
    STATUS="error"
    ERROR_TYPE=$(echo "$HEALTH_RESPONSE" | jq -r '.error.type' | jq -sRr @uri)
    ERROR_MESSAGE=$(echo "$HEALTH_RESPONSE" | jq -r '.error.message' | jq -sRr @uri)
    ERROR_CODE=$(echo "$HEALTH_RESPONSE" | jq -r '.error.code' | jq -sRr @uri)

    PARAMS="$PARAMS&status=$STATUS"
    PARAMS="$PARAMS&error_type=$ERROR_TYPE"
    PARAMS="$PARAMS&error_message=$ERROR_MESSAGE"
    PARAMS="$PARAMS&error_code=$ERROR_CODE"
else
    STATUS="success"
    for key in $(echo "$HEALTH_RESPONSE" | jq -r 'keys_unsorted[]'); do
        value=$(echo "$HEALTH_RESPONSE" | jq -r --arg key "$key" '.[$key]' | jq -sRr @uri)
        PARAMS="$PARAMS&$key=$value"
    done
fi

# Send GET request to the webhook
curl -G "$WEBHOOK_URL?$PARAMS"

echo "Webhook notification sent."