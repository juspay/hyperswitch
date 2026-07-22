#!/usr/bin/env bash
# test-api.sh — Validate Hyperswitch skills against the sandbox API (V1)
#
# Usage:
#   export HYPERSWITCH_API_KEY=snd_...
#   ./test-api.sh
#
# Requirements: curl, jq

set -euo pipefail

BASE_URL="${HYPERSWITCH_BASE_URL:-https://sandbox.hyperswitch.io}"
API_KEY="${HYPERSWITCH_API_KEY:?Set HYPERSWITCH_API_KEY environment variable}"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0

pass() { echo -e "${GREEN}✅ PASS${NC} — $1"; ((PASS++)); }
fail() { echo -e "${RED}❌ FAIL${NC} — $1"; ((FAIL++)); }
info() { echo -e "${YELLOW}ℹ️  INFO${NC} — $1"; }

hs_request() {
  local method="$1"
  local path="$2"
  local body="${3:-}"

  if [[ -n "$body" ]]; then
    curl -s --request "$method" \
      --url "${BASE_URL}${path}" \
      --header "Content-Type: application/json" \
      --header "api-key: ${API_KEY}" \
      --data "$body"
  else
    curl -s --request "$method" \
      --url "${BASE_URL}${path}" \
      --header "api-key: ${API_KEY}"
  fi
}

echo ""
echo "========================================"
echo " Hyperswitch Skills — API Test Suite"
echo " Base URL: ${BASE_URL}"
echo "========================================"
echo ""

# ──────────────────────────────────────────
# TEST 1: Create payment (immediate capture)
# ──────────────────────────────────────────
echo "--- Test 1: Create payment (automatic capture) ---"
PAYMENT=$(hs_request POST /payments '{
  "amount": 1000,
  "currency": "USD",
  "confirm": true,
  "capture_method": "automatic",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  },
  "return_url": "https://example.com/complete"
}')

PAYMENT_ID=$(echo "$PAYMENT" | jq -r '.payment_id // empty')
STATUS=$(echo "$PAYMENT" | jq -r '.status // empty')

if [[ "$STATUS" == "succeeded" ]]; then
  pass "POST /payments → status: succeeded (id: $PAYMENT_ID)"
else
  fail "POST /payments → expected 'succeeded', got '$STATUS'"
  echo "$PAYMENT" | jq .
fi

# ──────────────────────────────────────────
# TEST 2: Retrieve payment
# ──────────────────────────────────────────
echo ""
echo "--- Test 2: Retrieve payment ---"
if [[ -n "$PAYMENT_ID" ]]; then
  RETRIEVED=$(hs_request GET "/payments/${PAYMENT_ID}")
  RET_STATUS=$(echo "$RETRIEVED" | jq -r '.status // empty')
  if [[ "$RET_STATUS" == "succeeded" ]]; then
    pass "GET /payments/${PAYMENT_ID} → status: succeeded"
  else
    fail "GET /payments/${PAYMENT_ID} → expected 'succeeded', got '$RET_STATUS'"
  fi
else
  fail "GET /payments — skipped (no payment_id from Test 1)"
fi

# ──────────────────────────────────────────
# TEST 3: Auth-only payment (manual capture)
# ──────────────────────────────────────────
echo ""
echo "--- Test 3: Create payment (manual capture) ---"
AUTH_PAYMENT=$(hs_request POST /payments '{
  "amount": 2000,
  "currency": "USD",
  "confirm": true,
  "capture_method": "manual",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  },
  "return_url": "https://example.com/complete"
}')

AUTH_PAYMENT_ID=$(echo "$AUTH_PAYMENT" | jq -r '.payment_id // empty')
AUTH_STATUS=$(echo "$AUTH_PAYMENT" | jq -r '.status // empty')

if [[ "$AUTH_STATUS" == "requires_capture" ]]; then
  pass "POST /payments (capture_method: manual) → status: requires_capture"
else
  fail "POST /payments (manual) → expected 'requires_capture', got '$AUTH_STATUS'"
fi

# ──────────────────────────────────────────
# TEST 4: Capture payment
# ──────────────────────────────────────────
echo ""
echo "--- Test 4: Capture payment ---"
if [[ -n "$AUTH_PAYMENT_ID" && "$AUTH_STATUS" == "requires_capture" ]]; then
  CAPTURE=$(hs_request POST "/payments/${AUTH_PAYMENT_ID}/capture" '{}')
  CAP_STATUS=$(echo "$CAPTURE" | jq -r '.status // empty')
  if [[ "$CAP_STATUS" == "succeeded" ]]; then
    pass "POST /payments/${AUTH_PAYMENT_ID}/capture → status: succeeded"
  else
    fail "Capture → expected 'succeeded', got '$CAP_STATUS'"
    echo "$CAPTURE" | jq .
  fi
else
  fail "Capture — skipped (no auth payment_id)"
fi

# ──────────────────────────────────────────
# TEST 5: Create refund
# ──────────────────────────────────────────
echo ""
echo "--- Test 5: Create refund ---"
if [[ -n "$PAYMENT_ID" ]]; then
  REFUND=$(hs_request POST /refunds "{
    \"payment_id\": \"${PAYMENT_ID}\",
    \"amount\": 500,
    \"reason\": \"customer_request\"
  }")

  REFUND_ID=$(echo "$REFUND" | jq -r '.refund_id // empty')
  REFUND_STATUS=$(echo "$REFUND" | jq -r '.status // empty')

  if [[ "$REFUND_STATUS" == "pending" || "$REFUND_STATUS" == "succeeded" ]]; then
    pass "POST /refunds → status: ${REFUND_STATUS} (id: $REFUND_ID)"
  else
    fail "POST /refunds → expected 'pending' or 'succeeded', got '$REFUND_STATUS'"
    echo "$REFUND" | jq .
  fi
else
  fail "POST /refunds — skipped (no payment_id)"
fi

# ──────────────────────────────────────────
# TEST 6: Retrieve refund
# ──────────────────────────────────────────
echo ""
echo "--- Test 6: Retrieve refund ---"
if [[ -n "${REFUND_ID:-}" ]]; then
  RETRIEVED_REFUND=$(hs_request GET "/refunds/${REFUND_ID}")
  RET_REFUND_STATUS=$(echo "$RETRIEVED_REFUND" | jq -r '.status // empty')
  if [[ "$RET_REFUND_STATUS" == "pending" || "$RET_REFUND_STATUS" == "succeeded" ]]; then
    pass "GET /refunds/${REFUND_ID} → status: ${RET_REFUND_STATUS}"
  else
    fail "GET /refunds/${REFUND_ID} → unexpected status: '$RET_REFUND_STATUS'"
  fi
else
  fail "GET /refunds — skipped (no refund_id)"
fi

# ──────────────────────────────────────────
# TEST 7: Cancel payment
# ──────────────────────────────────────────
echo ""
echo "--- Test 7: Cancel payment ---"
CANCEL_PAYMENT=$(hs_request POST /payments '{
  "amount": 1500,
  "currency": "USD",
  "confirm": true,
  "capture_method": "manual",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  },
  "return_url": "https://example.com/complete"
}')
CANCEL_ID=$(echo "$CANCEL_PAYMENT" | jq -r '.payment_id // empty')
CANCEL_PRE_STATUS=$(echo "$CANCEL_PAYMENT" | jq -r '.status // empty')

if [[ "$CANCEL_PRE_STATUS" == "requires_capture" && -n "$CANCEL_ID" ]]; then
  CANCELLED=$(hs_request POST "/payments/${CANCEL_ID}/cancel" '{"cancellation_reason": "requested_by_customer"}')
  CANCELLED_STATUS=$(echo "$CANCELLED" | jq -r '.status // empty')
  if [[ "$CANCELLED_STATUS" == "cancelled" ]]; then
    pass "POST /payments/${CANCEL_ID}/cancel → status: cancelled"
  else
    fail "Cancel → expected 'cancelled', got '$CANCELLED_STATUS'"
  fi
else
  fail "Cancel — skipped (could not create auth-only payment)"
fi

# ──────────────────────────────────────────
# TEST 8: List payments
# ──────────────────────────────────────────
echo ""
echo "--- Test 8: List payments ---"
LIST=$(hs_request GET "/payments/list?limit=5")
COUNT=$(echo "$LIST" | jq -r '.count // empty')
if [[ -n "$COUNT" ]]; then
  pass "GET /payments/list → returned $COUNT payments"
else
  fail "GET /payments/list → unexpected response"
  echo "$LIST" | jq . | head -20
fi

# ──────────────────────────────────────────
# TEST 9: Create payment link
# ──────────────────────────────────────────
echo ""
echo "--- Test 9: Create payment link ---"
PLINK=$(hs_request POST /payment_links '{
  "amount": 5000,
  "currency": "USD",
  "description": "Skills test payment link"
}')
PLINK_ID=$(echo "$PLINK" | jq -r '.payment_link_id // empty')
PLINK_URL=$(echo "$PLINK" | jq -r '.link // empty')

if [[ -n "$PLINK_URL" ]]; then
  pass "POST /payment_links → link: $PLINK_URL"
else
  fail "POST /payment_links → no link returned"
  echo "$PLINK" | jq .
fi

# ──────────────────────────────────────────
# SUMMARY
# ──────────────────────────────────────────
echo ""
echo "========================================"
echo " Results: ${PASS} passed, ${FAIL} failed"
echo "========================================"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
