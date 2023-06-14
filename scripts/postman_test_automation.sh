#!/bin/sh

# [PATH DECLARATIONS] ----------------------------------------------------------
# TOML_FILE=$HOME/target/test/connector_auth.toml
TOML_FILE=crates/router/tests/connectors/auth.toml

ADMIN_API_KEY=""
BASE_URL=""
MERCHANT_ID=""

API_KEY=""
API_SECRET=""
KEY1=""

CERTIFICATE=""
CERTIFICATE_KEY=""

CONNECTOR_NAME=(
    "aci"
    "adyen"
    "stripe"
)
JSON_PATH=(
    "/postman/aci.postman_collection.json"
    "/postman/adyen.postman_collection.json"
    "/postman/stripe.postman_collection.json"
)

# [FUNCTION DECLARATIONS] --------------------------------------------------------
# Get key-value pair
kv_pair() {
    local key=$1
    local index=-1

    # Find the index of the key
    for i in "${!CONNECTOR_NAME[@]}"; do
        if [[ ${CONNECTOR_NAME[$i]} == $key ]]; then
            index=$i
            break
        fi
    done

    # Check if the key exists and retrieve the corresponding value
    if [[ $index != -1 ]]; then
        value="${JSON_PATH[$index]}"
        echo "Key: $key, Value: $value"
    else
        echo "Key not found: $key"
    fi

    # Iterate over the arrays
    # for i in "${!CONNECTOR_NAME[@]}"; do
    #     key="${CONNECTOR_NAME[$i]}"
    #     value="${JSON_PATH[$i]}"
    #     echo "Key: $key, Value: $value"
    #     # echo -e $COLLECTION
    # done
}

# Get the API Keys
get_val() {
  input=$1
  awk -v name="$input" -F ' // ' 'BEGIN{ flag=0 } /^\[.*\]/{ if ($1 == "["name"]") { flag=1 } else { flag=0 } } flag==1 && /^[^#]/ { print $0 }' $TOML_FILE
}

# [RUNNER] -----------------------------------------------------------------------
for i in "${!CONNECTOR_NAME[@]}"; do
    x_val=$(get_val "${CONNECTOR_NAME[$i]}")
    echo "$x_val"
    # fix needed
    newman run postman/hyperswitch.postman_collection.json --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var gateway_merchant_id=$MERCHANT_ID --env-var certificate=$CERTIFICATE --env-var certificate_keys=$CERTIFICATE_KEY

done