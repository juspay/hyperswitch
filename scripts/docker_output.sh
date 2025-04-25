#! /usr/bin/env bash
set -euo pipefail

# Define the URL to check service availability (adjust HOST and PORT if needed)
HOST="localhost"
PORT="8080"
SERVICE_URL="http://${HOST}:${PORT}/health"

MAX_RETRIES=70
RETRY_COUNT=0

# Wait until the service is available or retries are exhausted
while ! curl --silent --fail "${SERVICE_URL}" > /dev/null; do
    if (( RETRY_COUNT >= MAX_RETRIES )); then
        echo ""
        echo "Service failed to start. Kindly check the logs."
        echo "You can view the logs using the command: docker-compose logs -f <service name>"
        exit 1
    fi
    printf "."
    sleep 2
    ((RETRY_COUNT++))
done

# Define ANSI 24-bit (true color) escape sequences for Light Sky Blue
LIGHT_SKY_BLUE="\033[38;2;135;206;250m"
RESET="\033[0m"

# Print the service URLs with only the links colored
echo -e "Control Centre running at ${LIGHT_SKY_BLUE}http://localhost:9000${RESET}"
echo -e "App server running at ${LIGHT_SKY_BLUE}http://localhost:8080${RESET}"
echo -e "Web-SDK running at ${LIGHT_SKY_BLUE}http://localhost:5252/HyperLoader.js${RESET}"
echo -e "Mailhog running at ${LIGHT_SKY_BLUE}http://localhost:8025${RESET}"