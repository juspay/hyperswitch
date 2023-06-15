#!/bin/bash

# [ DECLARATIONS ]
KEY_TYPE=""

API_KEY=""
API_SECRET=""
KEY1=""

# Unused as of now, will be useful once we start using this properly
CERTIFICATE=$4
CERTIFICATE_KEY=$5

path_generation() {
    local NAME=$1
    local COLLECTION_NAME=$"{\"$NAME\":\"postman/$NAME.postman_collection.json\"}"
    echo $COLLECTION_NAME | jq --arg v "$NAME" '.[$v]'  | tr -d '"'
}

get_api_keys() {
    local INPUT=$1
    RESULT=$(awk -v name="$INPUT" -F ' // ' 'BEGIN{ flag=0 } /^\[.*\]/{ if ($1 == "["name"]") { flag=1 } else { flag=0 } } flag==1 && /^[^#]/ { print $0 }' "$CONNECTOR_CONFIG_PATH")

    API_KEY=$(echo "$RESULT" | awk -F ' = ' '$1 == "api_key" { print $2 }')
    KEY1=$(echo "$RESULT" | awk -F ' = ' '$1 == "key1" { print $2 }')
    API_SECRET=$(echo "$RESULT" | awk -F ' = ' '$1 == "api_secret" { print $2 }')

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

get_api_keys "$CONNECTOR_NAME" > /dev/null
COLLECTION_PATH=$(path_generation $CONNECTOR_NAME)

if [[ "$KEY_TYPE" == "HeaderKey" ]]; then
    newman run $COLLECTION_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY
elif [[ "$KEY_TYPE" == "BodyKey" ]]; then
    newman run $COLLECTION_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var connector_key1=$KEY1
elif [[ "$KEY_TYPE" == "SignatureKey" ]]; then
    newman run $COLLECTION_PATH --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var connector_api_secret=$API_SECRET --env-var connector_key1=$KEY1
fi
