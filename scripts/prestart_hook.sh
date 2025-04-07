#! /usr/bin/env sh
set -euo pipefail

# Define the URL and parameters
PLATFORM="docker"
WEBHOOK_URL="https://hyperswitch.gateway.scarf.sh/${PLATFORM}"
VERSION="unknown"
STATUS="initiated"

# Send the GET request
curl --get "${WEBHOOK_URL}" --data-urlencode "version=${VERSION}" --data-urlencode "status=${STATUS}"

# Print confirmation
echo "Request sent to $WEBHOOK_URL with version=$VERSION and status=$STATUS"