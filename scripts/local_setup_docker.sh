#!/bin/bash
set -e

# Remove the existing Hyperswitch repository if it already exists
if [ -d "hyperswitch" ]; then
    echo "Removing existing Hyperswitch repository..."
    rm -rf hyperswitch
fi

# Clone the Hyperswitch repository
echo "Cloning Hyperswitch repository..."
git clone --depth 1 --branch latest https://github.com/juspay/hyperswitch

# Change directory into the repository folder
cd hyperswitch

# Start the containers in detached mode
echo "Starting Hyperswitch services with Docker Compose..."
docker compose up -d

# Define the URL to check for service availability.
# Adjust HOST and PORT if your setup exposes services on a different endpoint.
HOST="localhost"
PORT="8080"
SERVICE_URL="http://${HOST}:${PORT}/health"

# Wait for the service to be available. This loop uses curl to check if the service responds.
echo "Waiting for the services to be accessible at ${SERVICE_URL}..."
while ! curl --silent --fail "${SERVICE_URL}" > /dev/null; do
    printf "."
    sleep 2
done

echo ""
echo "All services are up and running."

# Display specific service URLs
echo "Hyperswitch App server running at http://localhost:8080"
echo "Hyperswitch Control Centre running at http://localhost:9000"
echo "Hyperswitch Web running at http://localhost:5252"
echo "Hyperswitch Mailhog running at http://localhost:8025"
echo "Hyperswitch PostgreSQL running at localhost:5432 (no web interface)"
echo "Hyperswitch Redis running at localhost:6379 (no web interface)"
