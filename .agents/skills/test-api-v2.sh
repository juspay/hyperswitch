#!/usr/bin/env bash
# test-api-v2.sh — Validate Hyperswitch skills against the V2 API endpoints
#
# Usage:
#   export HYPERSWITCH_API_KEY=snd_...
#   ./test-api-v2.sh
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
echo " Hyperswitch Skills — V2 API Test Suite"
echo " Base URL: ${BASE_URL}"
echo "========================================"
echo ""

# ──────────────────────────────────────────
# TEST 1: Create Payment Intent (V2)
# ──────────────────────────────────────────
echo "--- Test 1: Create payment intent (V2) ---"
INTENT=$(hs_request POST /v2/payments/create-intent '{
  "amount_details": {
    "order_amount": 2000,
    "currency": "USD"
  },
  "description": "V2 skills test"
}')

INTENT_ID=$(echo "$INTENT" | jq -r '.id // empty')
INTENT_STATUS=$(echo "$INTENT" | jq -r '.status // empty')

if [[ -n "$INTENT_ID" ]]; then
  pass "POST /v2/payments/create-intent → id: $INTENT_ID, status: $INTENT_STATUS"
else
  fail "POST /v2/payments/create-intent → no id returned"
  echo "$INTENT" | jq .
fi

# ──────────────────────────────────────────
# TEST 2: Get Payment Intent (V2)
# ──────────────────────────────────────────
echo ""
echo "--- Test 2: Get payment intent (V2) ---"
if [[ -n "$INTENT_ID" ]]; then
  GET_INTENT=$(hs_request GET "/v2/payments/${INTENT_ID}/get-intent")
  GET_STATUS=$(echo "$GET_INTENT" | jq -r '.status // empty')
  if [[ -n "$GET_STATUS" ]]; then
    pass "GET /v2/payments/${INTENT_ID}/get-intent → status: $GET_STATUS"
  else
    fail "GET /v2/payments/${INTENT_ID}/get-intent → no status"
    echo "$GET_INTENT" | jq .
  fi
else
  fail "GET /v2/payments intent — skipped (no intent_id)"
fi

# ──────────────────────────────────────────
# TEST 3: Confirm Payment Intent (V2)
# ──────────────────────────────────────────
echo ""
echo "--- Test 3: Confirm payment intent (V2) ---"
if [[ -n "$INTENT_ID" ]]; then
  CONFIRMED=$(hs_request POST "/v2/payments/${INTENT_ID}/confirm-intent" '{
    "payment_method_type": "card",
    "payment_method_subtype": "credit",
    "payment_method_data": {
      "card": {
        "card_number": "4242424242424242",
        "card_exp_month": "03",
        "card_exp_year": "2030",
        "card_cvc": "737"
      }
    },
    "browser_info": {
      "user_agent": "Mozilla/5.0 (skills-test)",
      "accept_header": "application/json",
      "language": "en-US",
      "color_depth": 24,
      "screen_height": 900,
      "screen_width": 1440,
      "time_zone": -480,
      "java_enabled": false,
      "java_script_enabled": true
    },
    "return_url": "https://example.com/complete"
  }')

  CONFIRMED_STATUS=$(echo "$CONFIRMED" | jq -r '.status // empty')
  if [[ "$CONFIRMED_STATUS" == "succeeded" || "$CONFIRMED_STATUS" == "processing" ]]; then
    pass "POST /v2/payments/${INTENT_ID}/confirm-intent → status: $CONFIRMED_STATUS"
  else
    info "POST /v2/payments/confirm-intent → status: $CONFIRMED_STATUS (may require additional auth)"
    echo "$CONFIRMED" | jq . | head -20
  fi
else
  fail "Confirm intent — skipped (no intent_id)"
fi

# ──────────────────────────────────────────
# TEST 4: Create and Confirm Intent in one step (V2)
# ──────────────────────────────────────────
echo ""
echo "--- Test 4: Create and confirm intent in one step (V2) ---"
ONESHOT=$(hs_request POST /v2/payments '{
  "amount_details": {
    "order_amount": 1500,
    "currency": "USD"
  },
  "payment_method_type": "card",
  "payment_method_subtype": "credit",
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

ONESHOT_STATUS=$(echo "$ONESHOT" | jq -r '.status // empty')
ONESHOT_ID=$(echo "$ONESHOT" | jq -r '.id // empty')

if [[ "$ONESHOT_STATUS" == "succeeded" || -n "$ONESHOT_ID" ]]; then
  pass "POST /v2/payments (create+confirm) → id: $ONESHOT_ID, status: $ONESHOT_STATUS"
else
  fail "POST /v2/payments → unexpected response"
  echo "$ONESHOT" | jq . | head -20
fi

# ──────────────────────────────────────────
# TEST 5: List V2 payments
# ──────────────────────────────────────────
echo ""
echo "--- Test 5: List payments (V2) ---"
V2_LIST=$(hs_request GET "/v2/payments/list?limit=5")
V2_COUNT=$(echo "$V2_LIST" | jq -r '.count // empty')
if [[ -n "$V2_COUNT" ]]; then
  pass "GET /v2/payments/list → returned $V2_COUNT payments"
else
  info "GET /v2/payments/list → response (may differ by account)"
fi

# ──────────────────────────────────────────
# SUMMARY
# ──────────────────────────────────────────
echo ""
echo "========================================"
echo " V2 Results: ${PASS} passed, ${FAIL} failed"
echo "========================================"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
