#! /usr/bin/env bash
set -euo pipefail

# Define the URL and parameters
SCARF_URL="https://hyperswitch.gateway.scarf.sh/docker"
VERSION=$1
INSTALLATION_STATUS=$2

CURL_COMMAND=("curl" "--get" "${SCARF_URL}" "--data-urlencode" "${VERSION}" "--data-urlencode" "${INSTALLATION_STATUS}")

# Calculate number of arguments and process remaining args (if any)
if [ $# -gt 2 ]; then
    # Starting from the 3rd argument (index 2 in $@)
    for param in "${@:3}"; do
        CURL_COMMAND+=("--data-urlencode" "${param}")
    done
fi

# Execute the curl command
echo "Executing: ${CURL_COMMAND[@]}"
"${CURL_COMMAND[@]}"

# Print confirmation
echo "Request sent to ${SCARF_URL} with ${VERSION} and ${INSTALLATION_STATUS}"
