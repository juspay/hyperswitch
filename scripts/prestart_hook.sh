#!/bin/sh

# Define the URL and parameters
PLATFORM="docker"
URL="https://hyperswitch.gateway.scarf.sh/$PLATFORM"
VERSION="1.113.0"
STATUS="initiated"

# Send the GET request
curl -G "$URL" --data-urlencode "version=$VERSION" --data-urlencode "status=$STATUS"

# Print confirmation
echo "Request sent to $URL with version=$VERSION and status=$STATUS"