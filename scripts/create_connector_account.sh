connector=$(echo -e "$1" | awk '{print tolower($0)}')
merchant_id="$2"

required_connector="stripe"

help() {
    echo -e "Usage: create_connector.sh <connector-name> <merchant_id>"
    exit 2
}

if [ -z "$connector" ]; then
    echo "Please provide a connector"
    help
fi

if [ -z "$merchant_id" ]; then
    echo "Please provide a merchant_id"
    help
fi

read_keys() {
    local api_key=$(echo -e "${required_connector}_api_key")
    local key=$(echo -e "${required_connector}_key1")
    local key1=$(grep "^$api_key" keys.conf | awk -F: '{ split($0, array, ":"); print array[2]}'| xargs)
    local key2=$(grep "^$key" keys.conf | awk -F: '{ split($0, array, ":"); print array[2]}'| xargs)

    if [[ "$required_connector" == "stripe" ]]; then 
        local json="\"auth_type\": \"HeaderKey\", \"api_key\": \"$key1\""
        echo "$json"
    else 
        local json="\"auth_type\": \"BodyKey\", \"api_key\": \"$key1\", \"key1\": \"$key2\""
        echo "$json"
    fi
}

case "$connector" in 
    stripe) required_connector="stripe";;
    checkout) required_connector="checkout";;
    authorizedotnet) required_connector="authorizedotnet";;
    aci) required_connector="aci";;
    adyen) required_connector="adyen";;
    braintree) required_connector="braintree";;
    shift4) required_connector="shift4";;
    *) echo "This connector is not supported" 1>&2;exit 1;;
esac

keys="$(read_keys)"

json=$(echo '{
  "connector_type": "fiz_operations",
  "connector_name": "'$required_connector'",
  "connector_account_details": {
    '$keys'
  },
  "test_mode": false,
  "disabled": false,
  "payment_methods_enabled": [
    {
      "payment_method": "wallet",
      "payment_method_types": [
        "upi_collect",
        "upi_intent"
      ],
      "payment_method_issuers": [
        "labore magna ipsum",
        "aute"
      ],
      "payment_schemes": [
        "Discover",
        "Discover"
      ],
      "accepted_currencies": [
        "AED",
        "AED"
      ],
      "accepted_countries": [
        "in",
        "us"
      ],
      "minimum_amount": 1,
      "maximum_amount": 68607706,
      "recurring_enabled": true,
      "installment_payment_enabled": true
    }
  ],
  "metadata": {
    "city": "NY",
    "unit": "245"
  }
}')

update_merchant_account=$(echo '{
  "merchant_id": "'$merchant_id'",
  "merchant_name": "NewAge Retailer",
  "merchant_details": {
    "primary_contact_person": "John Test",
    "primary_email": "JohnTest@test.com",
    "primary_phone": "veniam aute officia ullamco esse",
    "secondary_contact_person": "John Test2",
    "secondary_email": "JohnTest2@test.com",
    "secondary_phone": "proident adipisicing officia nulla",
    "website": "www.example.com",
    "about_business": "Online Retail with a wide selection of organic products for North America",
    "address": {
      "line1": "Juspay Router",
      "line2": "Koramangala",
      "line3": "Stallion",
      "city": "Bangalore",
      "state": "Karnataka",
      "zip": "560095",
      "country": "IN"
    }
  },
  "return_url": "www.example.com/success",
  "webhook_details": {
    "webhook_version": "1.0.1",
    "webhook_username": "ekart_retail",
    "webhook_password": "password_ekart@123",
    "payment_created_enabled": true,
    "payment_succeeded_enabled": true,
    "payment_failed_enabled": true
  },
  "routing_algorithm": "custom",
  "custom_routing_rules": [
    {
      "payment_methods_incl": [
        "card",
        "card"
      ],
      "payment_methods_excl": [
        "card",
        "card"
      ],
      "payment_method_types_incl": [
        "credit"
      ],
      "payment_method_types_excl": [
        "credit"
      ],
      "payment_method_issuers_incl": [
        "Citibank",
        "JPMorgan"
      ],
      "payment_method_issuers_excl": [
        "Citibank",
        "JPMorgan"
      ],
      "countries_incl": [
        "US",
        "UK",
        "IN"
      ],
      "countries_excl": [
        "US",
        "UK",
        "IN"
      ],
      "currencies_incl": [
        "USD",
        "EUR"
      ],
      "currencies_excl": [
        "AED",
        "SGD"
      ],
      "metadata_filters_keys": [
        "payments.udf1",
        "payments.udf2"
      ],
      "metadata_filters_values": [
        "android",
        "Category_Electronics"
      ],
      "connectors_pecking_order": [
        "'$required_connector'"
      ],
      "connectors_traffic_weightage_key": [
        "stripe",
        "adyen",
        "brain_tree"
      ],
      "connectors_traffic_weightage_value": [
        50,
        30,
        20
      ]
    },
    {
      "connectors_pecking_order": [
        "'$required_connector'"
      ],
      "connectors_traffic_weightage_key": [
        "stripe",
        "adyen",
        "brain_tree"
      ],
      "connectors_traffic_weightage_value": [
        50,
        30,
        20
      ]
    }
  ],
  "metadata": {
    "city": "NY",
    "unit": "245"
  }
}')

resp=$(curl -s --location --request POST 'http://127.0.0.1:8080/account/'$merchant_id'/connectors' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: test_admin' \
--data-raw "$json")

resp=$(curl -s --location --request POST 'http://127.0.0.1:8080/accounts/'$merchant_id'' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: test_admin' \
--data-raw "$update_merchant_account")

echo -e "\033[1mYour Connector $connector for Merchant ID $merchant_id has been created\033[0m\n"
