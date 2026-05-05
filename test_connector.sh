#!/bin/bash

curl -s -X POST "http://localhost:8080/account/test_merchant_1778001211/connectors" \
-H "Content-Type: application/json" \
-H "api-key: test_admin" \
-d '{"connector_type":"payment_processor","connector_name":"adyen","connector_account_details":{"auth_type":"SignatureKey","api_key":"test_key","key1":"test","api_secret":"secret"},"payment_methods_enabled":[{"payment_method":"card","payment_method_types":[{"payment_method_type":"credit","card_networks":["Visa"]}]}],"test_mode":true}' | jq . 2>/dev/null || echo "Failed"