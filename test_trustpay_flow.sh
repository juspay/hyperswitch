#!/bin/bash
set -e

MERCHANT_API_KEY="dev_zpljZTRXMyJbnKS7z7GZzVljLS0rflAwoX4YDWZU1RhXi1O8slD2cKLCmE3s1khs"
CUSTOMER_ID="cus_WM1j6xVACFqDMJWY2qND"
BASE_URL="http://localhost:8080"

echo "=== API Testing: Trustpay Order Create Flow ==="
echo "Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# Step 1: Create Payment Intent
echo "=== Step 1: Create Payment Intent ==="
CREATE_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST "${BASE_URL}/payments" \
  -H "Content-Type: application/json" \
  -H "api-key: ${MERCHANT_API_KEY}" \
  -d "{
    \"amount\": 1000,
    \"currency\": \"USD\",
    \"customer_id\": \"${CUSTOMER_ID}\",
    \"description\": \"Test payment for trustpay\",
    \"capture_method\": \"automatic\"
  }" 2>&1)

HTTP_CODE=$(echo "$CREATE_RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
BODY=$(echo "$CREATE_RESPONSE" | grep -v "HTTP_CODE:")

echo "HTTP Status: $HTTP_CODE"
echo "Response: $BODY"
echo ""

PAYMENT_ID=$(echo "$BODY" | grep -o '"payment_id":"[^"]*"' | cut -d'"' -f4)
CLIENT_SECRET=$(echo "$BODY" | grep -o '"client_secret":"[^"]*"' | cut -d'"' -f4)

echo "Extracted payment_id: $PAYMENT_ID"
echo "Extracted client_secret: $CLIENT_SECRET"
echo ""

if [ -z "$PAYMENT_ID" ]; then
    echo "ERROR: Failed to extract payment_id"
    exit 1
fi

# Step 2: Confirm Payment
echo "=== Step 2: Confirm Payment ==="
CONFIRM_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST "${BASE_URL}/payments/${PAYMENT_ID}/confirm" \
  -H "Content-Type: application/json" \
  -H "api-key: ${MERCHANT_API_KEY}" \
  -d "{
    \"payment_method\": \"card\",
    \"payment_method_type\": \"credit\",
    \"payment_method_data\": {
      \"card\": {
        \"card_number\": \"4111111111111111\",
        \"card_exp_month\": \"12\",
        \"card_exp_year\": \"2030\",
        \"card_holder_name\": \"Test User\",
        \"card_cvc\": \"123\"
      }
    }
  }" 2>&1)

HTTP_CODE=$(echo "$CONFIRM_RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
BODY=$(echo "$CONFIRM_RESPONSE" | grep -v "HTTP_CODE:")

echo "HTTP Status: $HTTP_CODE"
echo "Response: $BODY"
echo ""

# Step 3: Retrieve Payment
echo "=== Step 3: Retrieve Payment ==="
RETRIEVE_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X GET "${BASE_URL}/payments/${PAYMENT_ID}?expand_attempts=true" \
  -H "api-key: ${MERCHANT_API_KEY}" 2>&1)

HTTP_CODE=$(echo "$RETRIEVE_RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
BODY=$(echo "$RETRIEVE_RESPONSE" | grep -v "HTTP_CODE:")

echo "HTTP Status: $HTTP_CODE"
echo "Response: $BODY"
echo ""

STATUS=$(echo "$BODY" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "Final Payment Status: $STATUS"
echo ""

echo "=== API Flow Complete ==="
echo "Payment ID: $PAYMENT_ID"
echo "Final Status: $STATUS"
