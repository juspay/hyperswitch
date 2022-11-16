merchant_id="merchant_$(date +"%s")"

resp=$(curl -s --location --request POST 'http://127.0.0.1:8080/accounts' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: test_admin' \
--data-raw '{
  "merchant_id": "'$merchant_id'",
  "merchant_name": "NewAge Retailer",
  "merchant_details": {
    "primary_contact_person": "John Test",
    "primary_email": "JohnTest@test.com",
    "primary_phone": "sunt laborum",
    "secondary_contact_person": "John Test2",
    "secondary_email": "JohnTest2@test.com",
    "secondary_phone": "cillum do dolor id",
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
        "credit",
        "credit"
      ],
      "payment_method_types_excl": [
        "credit",
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
        "checkout"
      ],
      "connectors_traffic_weightage_key": [
       "checkout"
      ],
      "connectors_traffic_weightage_value": [
        50,
        30,
        20
      ]
    },
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
        "credit",
        "credit"
      ],
      "payment_method_types_excl": [
        "credit",
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
        "checkout"
      ],
      "connectors_traffic_weightage_key": [
        "checkout"
      ],
      "connectors_traffic_weightage_value": [
        50,
        30,
        20
      ]
    }
  ],
  "sub_merchants_enabled": false,
  "metadata": {
    "city": "NY",
    "unit": "245"
  }
}')


api_key=$(echo "$resp" | grep "api_key" | awk -F: '{ gsub(/ /,"");split($0, array, ","); split(array[3],array,":");print array[2]}'| cut -d: -f2 | tr -d ' "')
merchant_id=$(echo "$resp" | grep "merchant_id" | awk -F: '{ gsub(/ /,"");split($0, array, ","); split(array[1],array,":");print array[2]}'| cut -d: -f2 | tr -d ' "')
echo -e "\033[1mInstructions:\n1.Use this new API key and Merchant ID to test ORCA in your dev environment (localhost:8080)\n2.Copy and securely store this new API key and Merchant ID in your system
\033[0m\nMerchant-ID: $merchant_id\nAPI-KEY: $api_key"
