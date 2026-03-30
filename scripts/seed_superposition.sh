#!/bin/bash
# Seed Superposition with default configuration for local development

set -euo pipefail

SUPERPOSITION_URL="${SUPERPOSITION_URL:-http://localhost:8081}"
SEED_FILE="${SEED_FILE:-./config/superposition_seed.toml}"
WORKSPACE_ID="${WORKSPACE_ID:-dev}"
ORG_ID="${ORG_ID:-localorg}"

echo "Seeding Superposition at $SUPERPOSITION_URL"
echo "Using seed file: $SEED_FILE"
echo "Workspace: $WORKSPACE_ID, Org: $ORG_ID"

# Wait for superposition to be ready
echo "Waiting for Superposition to be ready..."
for i in {1..30}; do
    if curl -s "$SUPERPOSITION_URL/health" > /dev/null 2>&1; then
        echo "Superposition is ready!"
        break
    fi
    echo "Waiting for Superposition... ($i/30)"
    sleep 2
done

# Check if seed file exists
if [ ! -f "$SEED_FILE" ]; then
    echo "Error: Seed file not found at $SEED_FILE"
    exit 1
fi

# Convert TOML seed file to JSON for processing
SEED_JSON=$(yq -p toml -o json '.' "$SEED_FILE")

# Seed dimensions
# dimensions are stored as a map keyed by dimension name:
#   [dimensions.<name>] with position, schema, description, change_reason
echo "Seeding dimensions..."
echo "$SEED_JSON" | jq -c '.dimensions | to_entries[] | {dimension: .key, position: .value.position, schema: .value.schema, description: .value.description, change_reason: .value.change_reason}' | while read -r dimension; do
    dim_name=$(echo "$dimension" | jq -r '.dimension')

    echo "Creating dimension: $dim_name"

    curl -s -X POST "$SUPERPOSITION_URL/dimension" \
        -H "Content-Type: application/json" \
        -H "x-org-id: $ORG_ID" \
        -H "x-workspace: $WORKSPACE_ID" \
        -d "$dimension" || echo "Dimension may already exist, continuing..."
done

# Seed default configs
# default-configs are stored as a map keyed by config key:
#   [default-configs.<key>] with value, schema, description, change_reason
echo "Seeding default configurations..."
echo "$SEED_JSON" | jq -c '."default-configs" | to_entries[] | {key: .key, value: .value.value, schema: .value.schema, description: .value.description, change_reason: .value.change_reason}' | while read -r config; do
    key=$(echo "$config" | jq -r '.key')

    echo "Setting default config: $key"

    curl -s -X POST "$SUPERPOSITION_URL/default-config" \
        -H "Content-Type: application/json" \
        -H "x-org-id: $ORG_ID" \
        -H "x-workspace: $WORKSPACE_ID" \
        -d "$config" || echo "Config may already exist, continuing..."
done

echo "Seeding complete!"
