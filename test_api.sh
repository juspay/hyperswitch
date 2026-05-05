#!/bin/bash
set -e

TIMESTAMP=$(date +%s)
MERCHANT_ID="test_merchant_${TIMESTAMP}"

echo "=== STEP 1: Create Merchant ==="
echo "Merchant ID: ${MERCHANT_ID}"

curl -s -X POST "http://localhost:8080/accounts" \
  -H "Content-Type: application/json" \
  -H "api-key: test_admin" \
  -d "{
    \"merchant_id\": \"${MERCHANT_ID}\",
    \"locker_id\": \"m0010\",
    \"merchant_name\": \"Test Merchant\",
    \"merchant_details\": {
      \"primary_contact_person\": \"Test User\",
      \"primary_email\": \"test@example.com\",
      \"primary_phone\": \"1234567890\",
      \"website\": \"https://example.com\",
      \"about_business\": \"Test\",
      \"address\": {
        \"line1\": \"123 Test St\", \"city\": \"San Francisco\",
        \"state\": \"California\", \"zip\": \"94122\", \"country\": \"US\",
        \"first_name\": \"Test\", \"last_name\": \"User\"
      }
    },
    \"webhook_details\": {
      \"webhook_version\": \"1.0.1\",
      \"webhook_username\": \"test\",
      \"webhook_password\": \"password123\",
      \"payment_created_enabled\": true,
      \"payment_succeeded_enabled\": true,
      \"payment_failed_enabled\": true
    },
    \"return_url\": \"https://example.com\",
    \"sub_merchants_enabled\": false,
    \"metadata\": { \"city\": \"NY\", \"unit\": \"1\" },
    \"primary_business_details\": [{ \"country\": \"US\", \"business\": \"default\" }]
  }" > /tmp/merchant_response.json

echo "Merchant Response:"
cat /tmp/merchant_response.json | jq . 2>/dev/null || cat /tmp/merchant_response.json
echo ""

echo "=== STEP 2: Create API Key ==="
curl -s -X POST "http://localhost:8080/api_keys/${MERCHANT_ID}" \
  -H "Content-Type: application/json" \
  -H "api-key: test_admin" \
  -d '{
    "name": "Test API Key",
    "description": "Test key for QA pipeline",
    "expiration": "2030-01-01T00:00:00Z"
  }' > /tmp/api_key_response.json

echo "API Key Response:"
cat /tmp/api_key_response.json | jq . 2>/dev/null || cat /tmp/api_key_response.json
echo ""

API_KEY=$(cat /tmp/api_key_response.json | jq -r '.api_key' 2>/dev/null)
echo "Extracted API_KEY: ${API_KEY}"
