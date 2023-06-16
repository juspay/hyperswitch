#! /usr/bin/env bash
set -euo pipefail

# Just by the name of the connector, this function generates the name of the collection
# Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/stripe.postman_collection.json
path_generation() {
    local name="${1}"
    local collection_name="postman/${name}.postman_collection.json"
    echo "${collection_name}"
}

# This function gets the api keys from the connector_auth.toml file
# Also determines the type of key (HeaderKey, BodyKey, SignatureKey) for the connector
get_api_keys() {
    local input="${1}"
    # We get $CONNECTOR_CONFIG_PATH from the GITHUB_ENV
    result=$(awk -v name="${input}" -F ' // ' 'BEGIN{ flag=0 } /^\[.*\]/{ if ($1 == "["name"]") { flag=1 } else { flag=0 } } flag==1 && /^[^#]/ { print $0 }' "${CONNECTOR_CONFIG_PATH}")
    # OUTPUT of result for `<connector_name>` that has `HeaderKey`:
    # [<connector_name>]
    # api_key = "HeadKey of <connector_name>"

    API_KEY=$(echo "${result}" | awk -F ' = ' '$1 == "api_key" { print $2 }')
    KEY1=$(echo "${result}" | awk -F ' = ' '$1 == "key1" { print $2 }')
    API_SECRET=$(echo "${result}" | awk -F ' = ' '$1 == "api_secret" { print $2 }')

    if [[ -n "${API_KEY}" && -z "${KEY1}" && -z "${API_SECRET}" ]]; then
        KEY_TYPE="HeaderKey"
    elif [[ -n "${API_KEY}" && -n "{$KEY1}" && -z "${API_SECRET}" ]]; then
        KEY_TYPE="BodyKey"
    elif [[ -n "${API_KEY}" && -n "${KEY1}" && -n "${API_SECRET}" ]]; then
        KEY_TYPE="SignatureKey"
    else
        KEY_TYPE="Invalid"
    fi
}

# [ MAIN ]
CONNECTOR_NAME="${1}"
KEY_TYPE=""

API_KEY=""
API_SECRET=""
KEY1=""

# Function call
get_api_keys "${CONNECTOR_NAME}"
COLLECTION_PATH=$(path_generation "${CONNECTOR_NAME}")

# Run Newman collection
case "$KEY_TYPE" in
    "HeaderKey" )
        args=(
            --env-var "admin_api_key=${ADMIN_API_KEY}"
            --env-var "baseUrl=${BASE_URL}"
            --env-var "connector_api_key=${API_KEY}"
        )
        [[ -n "$GATEWAY_MERCHANT_ID" ]] && args+=("--env-var" "gateway_merchant_id=${GATEWAY_MERCHANT_ID}")
        [[ -n "$GPAY_CERTIFICATE" ]] && args+=("--env-var" "certificate=${GPAY_CERTIFICATE}")
        [[ -n "$GPAY_CERTIFICATE_KEYS" ]] && args+=("--env-var" "certificate_keys=${GPAY_CERTIFICATE_KEYS}")
        newman run "${COLLECTION_PATH}" "${args[@]}"
        ;;
    "BodyKey" )
        args=(
            --env-var "admin_api_key=${ADMIN_API_KEY}"
            --env-var "baseUrl=${BASE_URL}"
            --env-var "connector_api_key=${API_KEY}"
            --env-var "connector_key1=${KEY1}"
        )
        [[ -n "$GATEWAY_MERCHANT_ID" ]] && args+=("--env-var" "gateway_merchant_id=${GATEWAY_MERCHANT_ID}")
        [[ -n "$GPAY_CERTIFICATE" ]] && args+=("--env-var" "certificate=${GPAY_CERTIFICATE}")
        [[ -n "$GPAY_CERTIFICATE_KEYS" ]] && args+=("--env-var" "certificate_keys=${GPAY_CERTIFICATE_KEYS}")
        newman run "${COLLECTION_PATH}" "${args[@]}"
        ;;
    "SignatureKey" )
        args=(
            --env-var "admin_api_key=${ADMIN_API_KEY}"
            --env-var "baseUrl=${BASE_URL}"
            --env-var "connector_api_key=${API_KEY}"
            --env-var "connector_api_secret=${API_SECRET}"
            --env-var "connector_key1=${KEY1}"
        )
        [[ -n "$GATEWAY_MERCHANT_ID" ]] && args+=("--env-var" "gateway_merchant_id=${GATEWAY_MERCHANT_ID}")
        [[ -n "$GPAY_CERTIFICATE" ]] && args+=("--env-var" "certificate=${GPAY_CERTIFICATE}")
        [[ -n "$GPAY_CERTIFICATE_KEYS" ]] && args+=("--env-var" "certificate_keys=${GPAY_CERTIFICATE_KEYS}")
        newman run "${COLLECTION_PATH}" "${args[@]}"
        ;;
esac
