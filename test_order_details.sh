#!/bin/bash
# Test script for OrderDetails re-verification

set -e

CYPRESS_BASEURL="${CYPRESS_BASEURL:-http://localhost:8080}"
CONNECTOR_AUTH_FILE="${CYPRESS_CONNECTOR_AUTH_FILE_PATH:-/tmp/connector_auth.json}"

ADMIN_KEY="test_admin"
TIMESTAMP=$(date +%s)
MERCHANT_ID="test_merchant_${TIMESTAMP}"

echo "=== API Testing Agent - Mode 2 Re-verification ==="
echo "Base URL: $CYPRESS_BASEURL"
echo "Merchant ID: $MERCHANT_ID"
echo ""

# Step 1: Create Merchant
echo "Step 1: Creating merchant account..."
MERCHANT_RESPONSE=$(curl -s -X POST "$CYPRESS_BASEURL/accounts" \
  -H "api-key: $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"merchant_id\": \"$MERCHANT_ID\",
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
  }")

echo "Merchant Response: $MERCHANT_RESPONSE" | jq .

# Step 2: Create API Key
echo ""
echo "Step 2: Creating merchant API key..."
API_KEY_RESPONSE=$(curl -s -X POST "$CYPRESS_BASEURL/api_keys/$MERCHANT_ID" \
  -H "api-key: $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test API Key",
    "description": "Test key for QA pipeline",
    "expiration": "2030-01-01T00:00:00Z"
  }')

MERCHANT_API_KEY=$(echo $API_KEY_RESPONSE | jq -r '.api_key')
echo "API Key Response: $API_KEY_RESPONSE" | jq .
echo "Merchant API Key: ${MERCHANT_API_KEY:0:20}..."

# Step 3: Create Customer
echo ""
echo "Step 3: Creating customer..."
CUSTOMER_RESPONSE=$(curl -s -X POST "$CYPRESS_BASEURL/customers" \
  -H "api-key: $MERCHANT_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "name": "Test Customer"
  }')

CUSTOMER_ID=$(echo $CUSTOMER_RESPONSE | jq -r '.customer_id')
echo "Customer Response: $CUSTOMER_RESPONSE" | jq .
echo "Customer ID: $CUSTOMER_ID"

# Step 4: Test 1 - Missing product_name error response structure
echo ""
echo "=========================================="
echo "TEST 1: Missing product_name validation error"
echo "Testing error response structure..."
echo "=========================================="

MISSING_PRODUCT_RESPONSE=$(curl -s -X POST "$CYPRESS_BASEURL/payments" \
  -H "api-key: $MERCHANT_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"amount\": 6000,
    \"currency\": \"USD\",
    \"confirm\": true,
    \"capture_method\": \"automatic\",
    \"customer_id\": \"$CUSTOMER_ID\",
    \"payment_method\": \"card\",
    \"payment_method_type\": \"credit\",
    \"authentication_type\": \"no_three_ds\",
    \"payment_method_data\": {
      \"card\": {
        \"card_number\": \"4111111111111111\",
        \"card_exp_month\": \"03\",
        \"card_exp_year\": \"30\",
        \"card_holder_name\": \"John Doe\",
        \"card_cvc\": \"737\"
      }
    },
    \"order_details\": [
      {
        \"quantity\": 1,
        \"amount\": 6000
      }
    ]
  }")

echo "Response for missing product_name:"
echo "$MISSING_PRODUCT_RESPONSE" | jq .

# Check the error structure
echo ""
echo "Error Structure Analysis:"
echo "$MISSING_PRODUCT_RESPONSE" | jq '{
  error_type_from_error_object: .error.error_type,
  error_type_from_type_field: .error.type,
  code: .error.code,
  message: .error.message,
  full_error: .error
}'

echo ""
echo "=========================================="
echo "API Trace Complete"
echo "=========================================="
