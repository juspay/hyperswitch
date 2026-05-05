#!/bin/bash
set -e

MERCHANT_ID="test_merchant_1778001211"

echo "Creating connector for merchant: ${MERCHANT_ID}"

CONNECTOR_RESP=$(curl -s -X POST "http://localhost:8080/account/${MERCHANT_ID}/connectors" \
  -H "Content-Type: application/json" \
  -H "api-key: test_admin" \
  -d '{
    "connector_type": "payment_processor",
    "connector_name": "stripe",
    "connector_account_details": {
      "auth_type": "HeaderKey",
      "api_key": "pk_test_example_key"
    },
    "payment_methods_enabled": [
      {
        "payment_method": "card",
        "payment_method_types": [
          {
            "payment_method_type": "credit",
            "card_networks": ["Visa", "Mastercard"],
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
          }
        ]
      }
    ],
    "test_mode": true,
    "disabled": false
  }')

echo "Connector Response:"
echo "$CONNECTOR_RESP" | jq . 2>/dev/null || echo "$CONNECTOR_RESP"
