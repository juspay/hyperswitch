#!/bin/sh

# [ DECLARATIONS ] -----------------------------------------------------------------------
# TOML_FILE=$HOME/target/test/connector_auth.toml
TOML_FILE=crates/router/tests/connectors/auth.toml
CONFIGS=configs.ini
# COLLECTION=""

ADMIN_API_KEY=""
BASE_URL=""
MERCHANT_ID=""

API_KEY=""
API_SECRET=""
KEY1=""

CERTIFICATE=$4
CERTIFICATE_KEY=$5

# Hard code as of now
COLLECTION_NAME="{
    \"stripe\":\"/postman/stripe.postman_collection.json\",
    \"adyen\":\"/postman/adyen.postman_collection.json\"
}"

# [COMMENTING OUT FOR NOW - WILL BE USED LATER ONCE NOMENCLATURE IS DECIDED]
# CONNECTOR_NAME=(
#     "aci"
#     "adyen"
#     "authorizedotnet"
#     "checkout"
#     "cybersource"
#     "shift4"
#     "worldpay"
#     "payu"
#     "globalpay"
#     "stripe"
# )

# [ FUNCTIONS ] -----------------------------------------------------------------------
# path_collection_generation() {
#     for index in "${!CONNECTOR_NAME[@]}"; do
#         key="${CONNECTOR_NAME[$index]}"
#         COLLECTION_ENTRY="\"$key\":\"$key.postman_collection.json\""
        
#         if [ -z "$COLLECTION" ]; then
#             COLLECTION="{ $COLLECTION_ENTRY"
#         else
#             COLLECTION="$COLLECTION, $COLLECTION_ENTRY"
#         fi
#     done

#     COLLECTION="$COLLECTION }"

#     echo $COLLECTION
# }

# COLLECTION_PATH=$(path_collection_generation)
# echo $COLLECTION_PATH | jq --arg v "$1" ".[$v]"

tmp_path_collection_generation() {
    INPUT=$1
    echo $COLLECTION_NAME | jq --arg v "$INPUT" '.[$v]'  | tr -d '"'
    echo $(get_api_keys $INPUT)
}

get_api_keys() {
  local input=$1
  result=$(awk -v name="$input" -F ' // ' 'BEGIN{ flag=0 } /^\[.*\]/{ if ($1 == "["name"]") { flag=1 } else { flag=0 } } flag==1 && /^[^#]/ { print $0 }' "$TOML_FILE")

  API_KEY=$(echo "$result" | awk -F ' = ' '$1 == "api_key" { print $2 }')
  KEY1=$(echo "$result" | awk -F ' = ' '$1 == "key1" { print $2 }')
  API_SECRET=$(echo "$result" | awk -F ' = ' '$1 == "api_secret" { print $2 }')
}

get_gh_secrets() {
  input=$CONFIGS
  ADMIN_API_KEY=$(awk 'NR==1 {print $0}' "$input")
  BASE_URL=$(awk 'NR==2 {print $0}' "$input")
  MERCHANT_ID=$(awk 'NR==3 {print $0}' "$input")
}

# [ MAIN ] -----------------------------------------------------------------------
CONNECTOR_NAME=$1
COLLECTOR_PATH="$(tmp_path_collection_generation $CONNECTOR_NAME)"
get_api_keys "$CONNECTOR_NAME" > /dev/null
get_gh_secrets > /dev/null
echo "run" $COLLECTOR_PATH "--env-var admin_api_key=" $ADMIN_API_KEY "--env-var baseUrl=" $BASE_URL "--env-var connector_api_key=" $API_KEY "--env-var connector_api_secret=" $API_SECRET "--env-var connector_key1=" $KEY1 "--env-var gateway_merchant_id=" $MERCHANT_ID

# for i in "${!CONNECTOR_NAME[@]}"; do
#     x_val=$(get_api_keys "${CONNECTOR_NAME[$i]}")
#     echo "$x_val"
    # fix needed
    # newman run postman/hyperswitch.postman_collection.json --env-var admin_api_key=$ADMIN_API_KEY --env-var baseUrl=$BASE_URL --env-var connector_api_key=$API_KEY --env-var gateway_merchant_id=$MERCHANT_ID --env-var certificate=$CERTIFICATE --env-var certificate_keys=$CERTIFICATE_KEY
# done
