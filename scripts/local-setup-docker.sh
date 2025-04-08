#!/bin/bash
set -e

# Define the URL to check service availability (adjust HOST and PORT if needed)
HOST="localhost"
PORT="8080"
SERVICE_URL="http://${HOST}:${PORT}/health"

# Wait until the service is available
echo "Waiting for services at ${SERVICE_URL}..."
while ! curl --silent --fail "${SERVICE_URL}" > /dev/null; do
    printf "."
    sleep 2
done

echo ""

# Define ANSI 24-bit (true color) escape sequences for Light Sky Blue
LIGHT_SKY_BLUE="\033[38;2;135;206;250m"
RESET="\033[0m"

# Print the service URLs with only the links colored
echo -e "Control Centre running at ${LIGHT_SKY_BLUE}http://localhost:9000${RESET}"
echo -e "App server running at ${LIGHT_SKY_BLUE}http://localhost:8080/health${RESET}"
echo -e "Web-SDK running at ${LIGHT_SKY_BLUE}http://localhost:5252/HyperLoader.js${RESET}"
echo -e "Mailhog running at ${LIGHT_SKY_BLUE}http://localhost:8025${RESET}"
echo -e "PostgreSQL running at ${LIGHT_SKY_BLUE}localhost:5432${RESET} (no web interface)"
echo -e "Redis running at ${LIGHT_SKY_BLUE}localhost:6379${RESET} (no web interface)"
