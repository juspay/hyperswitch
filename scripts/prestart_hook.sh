#! /usr/bin/env bash
set -euo pipefail

ONE_CLICK_SETUP="${ONE_CLICK_SETUP:-false}"

# Check if ONE_CLICK_SETUP is set to true; if so, skip execution
if [ "${ONE_CLICK_SETUP}" = "true" ]; then
    echo "ONE_CLICK_SETUP is true; skipping script execution."
    exit 0
fi

# Define the URL and parameters
SCARF_URL="https://hyperswitch.gateway.scarf.sh/only-docker"
VERSION="unknown"
STATUS="initiated"

# Send the GET request
curl --get "${SCARF_URL}" --data-urlencode "version=${VERSION}" --data-urlencode "status=${STATUS}"

# Print confirmation
echo "Request sent to ${SCARF_URL} with version=${VERSION} and status=${STATUS}"
