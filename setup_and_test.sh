#!/bin/bash
set -e

MERCHANT_ID="test_merchant_1778001211"
API_KEY="dev_0g2SEeHbf4Lixk3UmfLVYjUFtiV8GA9FudPKZgAROlyhr6C7wtXtBVUofzU4Pxz2"

# Log responses for analysis
echo "=== API Trace Log ===" > /tmp/api_trace.log

# Step 3: Create Customer
echo "=== STEP 3: Create Customer ==="
CUSTOMER_RESP=$(curl -s -X POST "http://localhost:8080/customers" \
  -H "Content-Type: application/json" \
  -H "api-key: ${API_KEY}" \
  -d '{
    "email": "test@example.com",
    "name": "Test Customer"
  }')

echo "$CUSTOMER_RESP" | jq . > /tmp/customer_response.json
CUSTOMER_ID=$(echo "$CUSTOMER_RESP" | jq -r '.customer_id' 2>/dev/null)
echo "Customer ID: ${CUSTOMER_ID}"

# Step 4: Test OrderDetails - Single Item
echo ""
echo "=== STEP 4: Create/Confirm Payment with Single Order Details ==="

curl -s -X POST "http://localhost:8080/payments" \
  -H "Content-Type: application/json" \
  -H "api-key: ${API_KEY}" \
  -d "{
    \"amount\": 6000,
    \"currency\": \"USD\",
    \"confirm\": true,
    \"capture_method\": \"automatic\",
    \"customer_id\": \"${CUSTOMER_ID}\",
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
        \"product_name\": \"Test Product\",
        \"quantity\": 1,
        \"amount\": 6000
      }
    ]
  }" > /tmp/orderdetails_single_response.json

echo "Single OrderDetails Response:"
cat /tmp/orderdetails_single_response.json | jq . 2>/dev/null || cat /tmp/orderdetails_single_response.json
PAYMENT_ID_SINGLE=$(cat /tmp/orderdetails_single_response.json | jq -r '.payment_id' 2>/dev/null)
echo "Payment ID: ${PAYMENT_ID_SINGLE}"

# Step 5: Test OrderDetails - Multiple Items
echo ""
echo "=== STEP 5: Create/Confirm Payment with Multiple Order Details ==="

curl -s -X POST "http://localhost:8080/payments" \
  -H "Content-Type: application/json" \
  -H "api-key: ${API_KEY}" \
  -d "{
    \"amount\": 10000,
    \"currency\": \"USD\",
    \"confirm\": true,
    \"capture_method\": \"automatic\",
    \"customer_id\": \"${CUSTOMER_ID}\",
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
        \"product_name\": \"Test Product 1\",
        \"quantity\": 2,
        \"amount\": 5000
      },
      {
        \"product_name\": \"Test Product 2\",
        \"quantity\": 1,
        \"amount\": 5000
      }
    ]
  }" > /tmp/orderdetails_multi_response.json

echo "Multiple OrderDetails Response:"
cat /tmp/orderdetails_multi_response.json | jq . 2>/dev/null || cat /tmp/orderdetails_multi_response.json
PAYMENT_ID_MULTI=$(cat /tmp/orderdetails_multi_response.json | jq -r '.payment_id' 2>/dev/null)
echo "Payment ID: ${PAYMENT_ID_MULTI}"

# Step 6: Test OrderDetails - Missing product_name (validation error)
echo ""
echo "=== STEP 6: Create/Confirm Payment with Missing product_name (Expect IR_06) ==="

curl -s -X POST "http://localhost:8080/payments" \
  -H "Content-Type: application/json" \
  -H "api-key: ${API_KEY}" \
  -d "{
    \"amount\": 6000,
    \"currency\": \"USD\",
    \"confirm\": true,
    \"capture_method\": \"automatic\",
    \"customer_id\": \"${CUSTOMER_ID}\",
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
  }" > /tmp/orderdetails_missing_response.json

echo "Missing product_name Response:"
cat /tmp/orderdetails_missing_response.json | jq . 2>/dev/null || cat /tmp/orderdetails_missing_response.json

echo ""
echo "=== Done ==="
echo "MERCHANT_ID: ${MERCHANT_ID}"
echo "API_KEY: ${API_KEY}"
echo "CUSTOMER_ID: ${CUSTOMER_ID}"
