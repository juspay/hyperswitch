#!/bin/bash

# [ DECLARATIONS ]
KEY_TYPE=""
# COLLECTION=""

# ADMIN_API_KEY=""
# BASE_URL=""
# MERCHANT_ID=""

API_KEY=""
API_SECRET=""
KEY1=""

# Unused as of now, will be useful once we start using this properly
CERTIFICATE=$4
CERTIFICATE_KEY=$5

# Hard coded for now, this will be replaced by the below function in the coming days
COLLECTION_NAME="{
    \"stripe\":\"postman/stripe.postman_collection.json\",
    \"adyen\":\"postman/adyen.postman_collection.json\"
}"

# [COMMENTING OUT FOR NOW - WILL BE USED LATER ONCE NOMENCLATURE IS DECIDED]
# [USAGE: Give the connector name below, put up the postman collection with the name as `<connector_name>.postman_collection.json`]
# [ FUNCTIONS ]
path_collection_generation() {
    COLLECTION_ENTRY="\"$CONNECTOR_NAME\":\"postman/$CONNECTOR_NAME.postman_collection.json\""
    
    if [ -z "$COLLECTION" ]; then
        COLLECTION="{ $COLLECTION_ENTRY"
    else
        COLLECTION="$COLLECTION, $COLLECTION_ENTRY"
    fi

    COLLECTION="$COLLECTION }"
    echo $COLLECTION
}

# COLLECTION_PATH=$(path_collection_generation)
# echo $COLLECTION_PATH | jq --arg v "$1" ".[$v]"

tmp_path_collection_generation() {
    INPUT=$1
    echo $COLLECTION_NAME | jq --arg v "$INPUT" '.[$v]'  | tr -d '"'
    echo $(get_api_keys $INPUT)
}

get_api_keys() {
    local input=$1
    result=$(awk -v name="$input" -F ' // ' 'BEGIN{ flag=0 } /^\[.*\]/{ if ($1 == "["name"]") { flag=1 } else { flag=0 } } flag==1 && /^[^#]/ { print $0 }' "$CONNECTOR_CONFIG_PATH")

    API_KEY=$(echo "$result" | awk -F ' = ' '$1 == "api_key" { print $2 }')
    KEY1=$(echo "$result" | awk -F ' = ' '$1 == "key1" { print $2 }')
    API_SECRET=$(echo "$result" | awk -F ' = ' '$1 == "api_secret" { print $2 }')

    if [[ -n "$API_KEY" && -z "$KEY1" && -z "$API_SECRET" ]]; then
        KEY_TYPE="HeaderKey"
    elif [[ -n "$API_KEY" && -n "$KEY1" && -z "$API_SECRET" ]]; then
        KEY_TYPE="BodyKey"
    elif [[ -n "$API_KEY" && -n "$KEY1" && -n "$API_SECRET" ]]; then
        KEY_TYPE="SignatureKey"
    else
        KEY_TYPE="Invalid"
    fi


}

# [ MAIN ]
CONNECTOR_NAME=$1
COLLECTOR_PATH="$(tmp_path_collection_generation $CONNECTOR_NAME)"

get_api_keys "$CONNECTOR_NAME" > /dev/null
path_collection_generation

if [[ "$KEY_TYPE" == "HeaderKey" ]]; then
    newman run $COLLECTOR_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var gateway_merchant_id=$MERCHANT_ID
elif [[ "$KEY_TYPE" == "BodyKey" ]]; then
    newman run $COLLECTOR_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var connector_key1=$KEY1 --env-var gateway_merchant_id=$MERCHANT_ID
elif [[ "$KEY_TYPE" == "SignatureKey" ]]; then
    newman run $COLLECTOR_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var connector_api_secret=$API_SECRET --env-var connector_key1=$KEY1 --env-var gateway_merchant_id=$MERCHANT_ID
fi

# for i in "${!CONNECTOR_NAME[@]}"; do
#     x_val=$(get_api_keys "${CONNECTOR_NAME[$i]}")
#     echo "$x_val"
    # fix needed
    # newman run postman/hyperswitch.postman_collection.json --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var gateway_merchant_id=$MERCHANT_ID --env-var certificate=$CERTIFICATE --env-var certificate_keys=$CERTIFICATE_KEY
# done
