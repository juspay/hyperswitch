#!/bin/bash
# Seed Superposition with default configuration for local development

set -euo pipefail

SUPERPOSITION_URL="${SUPERPOSITION_URL:-http://localhost:8081}"
SEED_FILE="${SEED_FILE:-./config/superposition_seed.json}"
WORKSPACE_ID="${WORKSPACE_ID:-dev}"
ORG_ID="${ORG_ID:-localorg}"
MAX_RETRIES="${MAX_RETRIES:-60}"
RETRY_INTERVAL="${RETRY_INTERVAL:-2}"

echo "Seeding Superposition at $SUPERPOSITION_URL"
echo "Using seed file: $SEED_FILE"
echo "Workspace: $WORKSPACE_ID, Org: $ORG_ID"

show_progress() {
    local current="$1"
    local total="$2"
    local prefix="$3"
    
    local width=40
    if [ "$total" -eq 0 ]; then return; fi
    local percent=$(( current * 100 / total ))
    local completed=$(( width * current / total ))
    local remaining=$(( width - completed ))
    
    printf "\r%s [" "$prefix"
    for ((i=0; i<completed; i++)); do printf "#"; done
    for ((i=0; i<remaining; i++)); do printf "-"; done
    printf "] %d%% (%d/%d)" "$percent" "$current" "$total"
    if [ "$current" -eq "$total" ]; then
        echo ""
    fi
}


# Wait for superposition to be ready
echo "Waiting for Superposition to be ready..."
READY=0
i=1
while [ "$i" -le "$MAX_RETRIES" ]; do
    if curl -sS -o /dev/null "$SUPERPOSITION_URL/health"; then
        echo "Superposition is ready!"
        READY=1
        break
    fi
    echo "Waiting for Superposition... ($i/$MAX_RETRIES)"
    sleep "$RETRY_INTERVAL"
    i=$((i + 1))
done

if [ "$READY" -ne 1 ]; then
    echo "Error: Superposition did not become ready at $SUPERPOSITION_URL after $((MAX_RETRIES * RETRY_INTERVAL))s"
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
            # Silently continue on 409 to not break progress bar
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

# Read JSON seed file for processing
SEED_JSON=$(cat "$SEED_FILE")

# Seed dimensions
# dimensions are stored as a map keyed by dimension name:
#   [dimensions.<name>] with position, schema, description, change_reason
echo "Seeding dimensions..."
TOTAL_DIMS=$(echo "$SEED_JSON" | jq '.dimensions | length')
CURRENT_DIM=0
echo "$SEED_JSON" | jq -c '.dimensions | to_entries | sort_by(.value.position) | .[] | {dimension: .key, position: .value.position, schema: .value.schema, description: (.value.description // "Dimension: \(.key)"), change_reason: (.value.change_reason // "Seeded from file")}' | while read -r dimension; do
    dim_name=$(echo "$dimension" | jq -r '.dimension')

    post_or_fail "$SUPERPOSITION_URL/dimension" "$dimension" "dimension $dim_name"
    CURRENT_DIM=$((CURRENT_DIM + 1))
    show_progress "$CURRENT_DIM" "$TOTAL_DIMS" "Dimensions"
done

# Seed default configs
# default-configs are stored as a map keyed by config key:
#   [default-configs.<key>] with value, schema, description, change_reason
echo "Seeding default configurations..."
TOTAL_CONFIGS=$(echo "$SEED_JSON" | jq '."default-configs" | length')
CURRENT_CONFIG=0
echo "$SEED_JSON" | jq -c '."default-configs" | to_entries[] | {key: .key, value: .value.value, schema: .value.schema, description: (.value.description // "Config: \(.key)"), change_reason: (.value.change_reason // "Seeded from file")}' | while read -r config; do
    key=$(echo "$config" | jq -r '.key')

    post_or_fail "$SUPERPOSITION_URL/default-config" "$config" "default-config $key"
    CURRENT_CONFIG=$((CURRENT_CONFIG + 1))
    show_progress "$CURRENT_CONFIG" "$TOTAL_CONFIGS" "Configs   "
done

echo "Seeding complete!"
