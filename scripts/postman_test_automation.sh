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

    # Keys are exported as environment variables since some API Keys for connectors such as ACI
    # which is Base64 based and requires "Bearer" to be prefixed such as "Bearer Skst45645gey5r#&$==".
    # This effectively stops the shell from interpreting the value of the variable as a command.
    API_KEY=$(echo "${result}" | awk -F ' = ' '$1 == "api_key" { gsub(/"/, "", $2); print $2 }')
    KEY1=$(echo "${result}" | awk -F ' = ' '$1 == "key1" { gsub(/"/, "", $2); print $2 }')
    KEY2=$(echo "${result}" | awk -F ' = ' '$1 == "key2" { gsub(/"/, "", $2); print $2 }')
    API_SECRET=$(echo "${result}" | awk -F ' = ' '$1 == "api_secret" { gsub(/"/, "", $2); print $2 }')
    
    # Determine the type of key
    if [[ -n "${API_KEY}" && -z "${KEY1}" && -z "${API_SECRET}" ]]; then
        KEY_TYPE="HeaderKey"
    elif [[ -n "${API_KEY}" && -n "${KEY1}" && -z "${API_SECRET}" ]]; then
        KEY_TYPE="BodyKey"
    elif [[ -n "${API_KEY}" && -n "${KEY1}" && -n "${API_SECRET}" ]]; then
        KEY_TYPE="SignatureKey"
    elif [[ -n "${API_KEY}" && -n "${KEY1}" && -n "${KEY2}" && -n "${API_SECRET}" ]]; then
        KEY_TYPE="MultiAuthKey"
    else
        KEY_TYPE="Invalid"
    fi
}

# [ MAIN ]
CONNECTOR_NAME="${1}"
KEY_TYPE=""

# Function call
COLLECTION_PATH=$(path_generation "${CONNECTOR_NAME}")
get_api_keys "${CONNECTOR_NAME}"

# Newman runner
# Depending on the conditions satisfied, variables are added. Since certificates of stripe have already
# been added to the postman collection, those conditions are set to true and collections that have
# variables set up for certificate, will consider those variables and will fail.
newman run "${COLLECTION_PATH}" \
    --env-var "admin_api_key=${ADMIN_API_KEY}" \
    --env-var "baseUrl=${BASE_URL}" \
    --env-var "connector_api_key=${API_KEY}" \
    $(if [[ "${KEY_TYPE}" == BodyKey ]]; then echo --env-var "connector_key1=${KEY1}"; fi) \
    $(if [[ "${KEY_TYPE}" == SignatureKey ]]; then echo --env-var "connector_key1=${KEY1}" --env-var "connector_api_secret=${API_SECRET}"; fi) \
    $(if [[ "${KEY_TYPE}" == MultiAuthKey ]]; then echo --env-var "connector_key1=${KEY1}" --env-var "connector_key2=${KEY2}" --env-var "connector_api_secret=${API_SECRET}"; fi) \
    $(if [[ -n "${GATEWAY_MERCHANT_ID}" ]]; then echo --env-var "gateway_merchant_id=${GATEWAY_MERCHANT_ID}"; fi) \
    $(if [[ -n "${GPAY_CERTIFICATE}" ]]; then echo --env-var "certificate=${GPAY_CERTIFICATE}"; fi) \
    $(if [[ -n "${GPAY_CERTIFICATE_KEYS}" ]]; then echo --env-var "certificate_keys=${GPAY_CERTIFICATE_KEYS}"; fi) \
    --delay-request 5
