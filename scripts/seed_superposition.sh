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
READY=0
for i in {1..60}; do
    if curl -sS -o /dev/null "$SUPERPOSITION_URL/health"; then
        echo "Superposition is ready!"
        READY=1
        break
    fi
    echo "Waiting for Superposition... ($i/60)"
    sleep 2
done

if [ "$READY" -ne 1 ]; then
    echo "Error: Superposition did not become ready at $SUPERPOSITION_URL after 120s"
    exit 1
fi

# Check if seed file exists
if [ ! -f "$SEED_FILE" ]; then
    echo "Error: Seed file not found at $SEED_FILE"
    exit 1
fi

# POST a payload and accept 2xx or 409 (already exists); fail loudly on anything else.
post_or_fail() {
    local url="$1"
    local payload="$2"
    local label="$3"

    local tmp
    tmp=$(mktemp)
    local status
    status=$(curl -sS -o "$tmp" -w "%{http_code}" -X POST "$url" \
        -H "Content-Type: application/json" \
        -H "x-org-id: $ORG_ID" \
        -H "x-workspace: $WORKSPACE_ID" \
        -d "$payload")

    case "$status" in
        2??)
            ;;
        409)
            echo "  $label already exists (HTTP 409), continuing"
            ;;
        *)
            echo "Error: $label failed with HTTP $status"
            echo "Response body:"
            cat "$tmp"
            rm -f "$tmp"
            exit 1
            ;;
    esac
    rm -f "$tmp"
}

# Convert TOML seed file to JSON for processing
SEED_JSON=$(yq -p toml -o json '.' "$SEED_FILE")

# Seed dimensions
# dimensions are stored as a map keyed by dimension name:
#   [dimensions.<name>] with position, schema, description, change_reason
echo "Seeding dimensions..."
echo "$SEED_JSON" | jq -c '.dimensions | to_entries[] | {dimension: .key, position: .value.position, schema: .value.schema, description: .value.description, change_reason: .value.change_reason}' | while read -r dimension; do
    dim_name=$(echo "$dimension" | jq -r '.dimension')

    echo "Creating dimension: $dim_name"
    post_or_fail "$SUPERPOSITION_URL/dimension" "$dimension" "dimension $dim_name"
done

# Seed default configs
# default-configs are stored as a map keyed by config key:
#   [default-configs.<key>] with value, schema, description, change_reason
echo "Seeding default configurations..."
echo "$SEED_JSON" | jq -c '."default-configs" | to_entries[] | {key: .key, value: .value.value, schema: .value.schema, description: .value.description, change_reason: .value.change_reason}' | while read -r config; do
    key=$(echo "$config" | jq -r '.key')

    echo "Setting default config: $key"
    post_or_fail "$SUPERPOSITION_URL/default-config" "$config" "default-config $key"
done

echo "Seeding complete!"
