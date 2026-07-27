#!/bin/bash
# Seed Superposition with default configuration for local development

set -euo pipefail

SUPERPOSITION_URL="${SUPERPOSITION_URL:-http://localhost:8081}"
SEED_FILE="${SEED_FILE:-./config/superposition_seed.toml}"
WORKSPACE_ID="${WORKSPACE_ID:-dev}"
ORG_ID="${ORG_ID:-localorg}"
MAX_RETRIES="${MAX_RETRIES:-60}"
RETRY_INTERVAL="${RETRY_INTERVAL:-2}"

echo "Seeding Superposition at $SUPERPOSITION_URL"
echo "Using seed file: $SEED_FILE"
echo "Workspace: $WORKSPACE_ID, Org: $ORG_ID"

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

# Seed the Deja recording sampler: cohort dimension + override
# ------------------------------------------------------------------------------
# `deja_dimension` is a LOCAL_COHORT on the `path` dimension: its json-logic
# classifies the RAW request path (substring `in`, so parametric paths like
# /payments/{id}/confirm are covered) into "recordable"/"otherwise". The override
# records the recordable bucket; `deja_record` defaults to false (seeded above),
# so /health and other probe traffic skip. The path -> deja_dimension dependency
# graph is derived server-side from the LOCAL_COHORT type.
# Posted here as explicit JSON (not via the TOML) because a cohort's
# dimension_type + json-logic definitions do not round-trip cleanly through yq.
echo "Seeding Deja recording sampler (deja_dimension cohort + deja_record override)..."
post_or_fail "$SUPERPOSITION_URL/dimension" '{
    "dimension": "deja_dimension",
    "position": 10,
    "dimension_type": { "LOCAL_COHORT": "path" },
    "description": "Deja request treatment class (path-derived cohort)",
    "change_reason": "Deja recording sampler",
    "schema": {
        "type": "string",
        "enum": ["recordable", "otherwise"],
        "definitions": {
            "recordable": { "or": [
                { "in": ["/payments", { "var": "path" }] },
                { "in": ["/accounts", { "var": "path" }] },
                { "in": ["/user/signup", { "var": "path" }] },
                { "in": ["/organization", { "var": "path" }] },
                { "in": ["/api_keys", { "var": "path" }] },
                { "in": ["/configs", { "var": "path" }] }
            ]}
        }
    }
}' "deja_dimension cohort"

# The override context is created via PUT /context (context = condition map,
# override = config values): deja_dimension == "recordable" -> deja_record = true.
echo "Creating deja_record override for the recordable cohort..."
ctx_tmp=$(mktemp)
ctx_status=$(curl -sS -o "$ctx_tmp" -w "%{http_code}" -X PUT "$SUPERPOSITION_URL/context" \
    -H "Content-Type: application/json" \
    -H "x-org-id: $ORG_ID" \
    -H "x-workspace: $WORKSPACE_ID" \
    -d '{
        "context": { "deja_dimension": "recordable" },
        "override": { "deja_record": true },
        "description": "Record the recordable endpoint bucket",
        "change_reason": "Deja recording sampler"
    }')
case "$ctx_status" in
    2??) ;;
    409) echo "  deja_record override already exists (HTTP 409), continuing" ;;
    *) echo "Error: deja_record override failed with HTTP $ctx_status"; cat "$ctx_tmp"; rm -f "$ctx_tmp"; exit 1 ;;
esac
rm -f "$ctx_tmp"

echo "Seeding complete!"
