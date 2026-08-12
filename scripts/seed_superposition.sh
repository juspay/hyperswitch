#!/bin/bash
# Seed Superposition with default configuration for local development

set -euo pipefail

SUPERPOSITION_URL="${SUPERPOSITION_URL:-http://localhost:8081}"
SEED_FILE="${SEED_FILE:-./config/superposition_seed.toml}"
WORKSPACE_ID="${WORKSPACE_ID:-dev}"
ORG_ID="${ORG_ID:-localorg}"
MAX_RETRIES="${MAX_RETRIES:-60}"
RETRY_INTERVAL="${RETRY_INTERVAL:-2}"

for dependency in curl jq; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
        echo "Error: Required command '$dependency' was not found in PATH"
        exit 127
    fi
done

toml_to_json() {
    if command -v yq >/dev/null 2>&1; then
        yq -p toml -o json '.' "$SEED_FILE"
    elif command -v python3 >/dev/null 2>&1 \
        && python3 -c 'import tomllib' >/dev/null 2>&1; then
        python3 -c \
            'import json, sys, tomllib; json.dump(tomllib.load(open(sys.argv[1], "rb")), sys.stdout)' \
            "$SEED_FILE"
    else
        echo "Error: TOML parsing requires yq or Python 3.11+" >&2
        exit 127
    fi
}

show_progress() {
    local current="$1"
    local total="$2"
    local label="$3"
    local width=40
    local percent
    local completed
    local remaining
    local progress_index

    if [ "$total" -eq 0 ]; then
        return
    fi

    percent=$((current * 100 / total))
    completed=$((width * current / total))
    remaining=$((width - completed))

    printf "\r%-18s [" "$label"
    for ((progress_index = 0; progress_index < completed; progress_index++)); do
        printf "#"
    done
    for ((progress_index = 0; progress_index < remaining; progress_index++)); do
        printf "-"
    done
    printf "] %3d%% (%d/%d)" "$percent" "$current" "$total"

    if [ "$current" -eq "$total" ]; then
        echo ""
    fi
}

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
            ;;
        *)
            echo ""
            echo "Error: $label failed with HTTP $status"
            echo "Response body:"
            cat "$tmp"
            rm -f "$tmp"
            exit 1
            ;;
    esac
    rm -f "$tmp"
}

# PUT a payload and accept 2xx or 409 (already exists); fail loudly on anything else.
put_or_fail() {
    local url="$1"
    local payload="$2"
    local label="$3"

    local tmp
    tmp=$(mktemp)
    local status
    status=$(curl -sS -o "$tmp" -w "%{http_code}" -X PUT "$url" \
        -H "Content-Type: application/json" \
        -H "x-org-id: $ORG_ID" \
        -H "x-workspace: $WORKSPACE_ID" \
        -d "$payload")

    case "$status" in
        2??)
            ;;
        409)
            ;;
        *)
            echo ""
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
SEED_JSON=$(toml_to_json)

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
    show_progress "$CURRENT_CONFIG" "$TOTAL_CONFIGS" "Default configs"
done

# Seed overrides. FileDataSource stores each context in `_context_`, with all
# remaining keys representing the config values overridden for that context.
echo "Seeding overrides..."
TOTAL_OVERRIDES=$(echo "$SEED_JSON" | jq '.overrides | length')
CURRENT_OVERRIDE=0
echo "$SEED_JSON" | jq -c '.overrides[] | {
    context: ._context_,
    override: del(._context_),
    description: "Override for context \(.["_context_"] | tojson)",
    change_reason: "Seeded from file"
}' | while read -r context_payload; do
    context=$(echo "$context_payload" | jq -c '.context')

    put_or_fail "$SUPERPOSITION_URL/context" "$context_payload" "override for context $context"
    CURRENT_OVERRIDE=$((CURRENT_OVERRIDE + 1))
    show_progress "$CURRENT_OVERRIDE" "$TOTAL_OVERRIDES" "Overrides"
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
# dimension_type + json-logic definitions do not round-trip cleanly through the
# TOML conversion.
echo "Seeding Deja recording sampler (deja_dimension cohort + deja_record override)..."
post_or_fail "$SUPERPOSITION_URL/dimension" '{
    "dimension": "deja_dimension",
    "position": 19,
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
put_or_fail "$SUPERPOSITION_URL/context" '{
    "context": { "deja_dimension": "recordable" },
    "override": { "deja_record": true },
    "description": "Record the recordable endpoint bucket",
    "change_reason": "Deja recording sampler"
}' "deja_record override"

echo "Seeding complete!"
