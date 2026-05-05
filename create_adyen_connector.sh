#!/bin/bash
set -e

MERCHANT_ID="test_merchant_1778001211"

echo "Creating ADYEN connector for merchant: ${MERCHANT_ID}"

CONNECTOR_RESP=$(curl -s -X POST "http://localhost:8080/account/${MERCHANT_ID}/connectors" \
  -H "Content-Type: application/json" \
  -H "api-key: test_admin" \
  -d '{
    "connector_type": "payment_processor",
    "connector_name": "adyen",
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "AQEqhmfxK43MaR1Hw0m/n3Q5qf3VYp5eHZJTfEA0SnT87rrwTHXDVGtJ+kfCEMFdWw2+5HzctViMSCJMYAc=-sNyhV/b3uZx5d38TcqtscjboxGoH4khiJHYuEuUJ5IQ=-i1i2%dW^xT(m?b+LC7$",
      "key1": "JuspayDEECOM",
      "api_secret": "AQEzgmDBbd+uOlwd9n6PxDJo8rXOaKhCAINLVnwY7G24jmdSuuL0Salp1G0BJE6opzqZqP6rEMFdWw2+5HzctViMSCJMYAc=-bn/JeFXqIxxfhhy67PE2sTZctbqzqe+fU0JprcbCEmE=-:M><zzc+t9Ne#2eb"
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
