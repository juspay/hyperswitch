#!/usr/bin/env bash
# End-to-end test suite: Platform (provider + connected processors) x modular payment methods
# x network tokenization profile toggle.
#
# Usage:
#   bash scripts/test_platform_modular_nt.sh
#
# Requires: curl, jq, a running router on $BASE_URL and superposition on $SUPERPOSITION_URL.

set -uo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
SUPERPOSITION_URL="${SUPERPOSITION_URL:-http://localhost:8081}"
ADMIN_API_KEY="${ADMIN_API_KEY:-test_admin}"
SP_ORG_ID="${SP_ORG_ID:-localorg}"
SP_WORKSPACE="${SP_WORKSPACE:-dev}"

STAMP="$(date +%s)"
OUT_DIR="${OUT_DIR:-./.platform_nt_test_${STAMP}}"
mkdir -p "$OUT_DIR"

PROVIDER_MID="plat_provider_${STAMP}"
PROC_A_MID="proc_nt_on_${STAMP}"
PROC_B_MID="proc_nt_off_${STAMP}"

PASS=0
FAIL=0

for dep in curl jq; do
    command -v "$dep" >/dev/null 2>&1 || { echo "Error: '$dep' not found in PATH"; exit 127; }
done

# ---------------------------------------------------------------- helpers ---

hr()   { printf '\n\033[1m%s\033[0m\n' "──────────────────────────────────────────── $* ────"; }
info() { printf '  \033[2m%s\033[0m\n' "$*"; }

# call <method> <url> <label> [curl args...] -> body on stdout, status in $LAST_STATUS
LAST_STATUS=""
call() {
    local method="$1" url="$2" label="$3"; shift 3
    local body_file="${OUT_DIR}/$(echo "$label" | tr ' /' '__').json"
    LAST_STATUS=$(curl -sS -o "$body_file" -w '%{http_code}' -X "$method" "$url" "$@")
    cat "$body_file"
}

# expect <expected_status> <label>
expect() {
    local want="$1" label="$2"
    if [ "$LAST_STATUS" = "$want" ]; then
        printf '  \033[32m✓ PASS\033[0m %-52s (HTTP %s)\n' "$label" "$LAST_STATUS"
        PASS=$((PASS + 1))
    else
        printf '  \033[31m✗ FAIL\033[0m %-52s (HTTP %s, wanted %s)\n' "$label" "$LAST_STATUS" "$want"
        FAIL=$((FAIL + 1))
    fi
}

# set_modular <true|false>  — superposition override on organization_id
set_modular() {
    local value="$1"
    curl -sS -o /dev/null -X PUT "${SUPERPOSITION_URL}/context" \
        -H 'Content-Type: application/json' \
        -H "x-org-id: ${SP_ORG_ID}" \
        -H "x-workspace: ${SP_WORKSPACE}" \
        -d "$(jq -nc --arg org "$ORG_ID" --argjson val "$value" '{
              context: { organization_id: $org },
              override: { "system.should_call_pm_modular_service": $val },
              description: "platform modular NT test suite",
              change_reason: "test suite"
            }')"
    info "modular service for org ${ORG_ID} -> ${value} (allow ~polling_interval for cache)"
}

# ============================================================== PHASE 1 =====
hr "PHASE 1  provider (platform) merchant"

PROVIDER=$(call POST "${BASE_URL}/accounts" "01_provider_create" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg mid "$PROVIDER_MID" '{
          merchant_id: $mid,
          merchant_name: "Platform Provider",
          merchant_account_type: "platform",
          primary_business_details: [{ country: "US", business: "default" }]
        }')")
expect 200 "create provider merchant (type=platform)"

ORG_ID=$(echo "$PROVIDER" | jq -r '.organization_id')
PROVIDER_PROFILE=$(echo "$PROVIDER" | jq -r '.default_profile // empty')
info "provider merchant_id : ${PROVIDER_MID}"
info "organization_id      : ${ORG_ID}"
info "provider profile     : ${PROVIDER_PROFILE}"

if [ -z "$ORG_ID" ] || [ "$ORG_ID" = "null" ]; then
    echo "Fatal: could not read organization_id — aborting."; exit 1
fi

# ============================================================== PHASE 2 =====
hr "PHASE 2  two connected (processor) merchants in the same org"

PROC_A=$(call POST "${BASE_URL}/accounts" "02_processor_a_create" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg mid "$PROC_A_MID" --arg org "$ORG_ID" '{
          merchant_id: $mid,
          merchant_name: "Processor A (NT enabled)",
          organization_id: $org,
          merchant_account_type: "connected",
          primary_business_details: [{ country: "US", business: "default" }]
        }')")
expect 200 "create processor A (type=connected)"
PROC_A_PROFILE=$(echo "$PROC_A" | jq -r '.default_profile // empty')

PROC_B=$(call POST "${BASE_URL}/accounts" "03_processor_b_create" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg mid "$PROC_B_MID" --arg org "$ORG_ID" '{
          merchant_id: $mid,
          merchant_name: "Processor B (NT disabled)",
          organization_id: $org,
          merchant_account_type: "connected",
          primary_business_details: [{ country: "US", business: "default" }]
        }')")
expect 200 "create processor B (type=connected)"
PROC_B_PROFILE=$(echo "$PROC_B" | jq -r '.default_profile // empty')

info "processor A profile : ${PROC_A_PROFILE}"
info "processor B profile : ${PROC_B_PROFILE}"

# ============================================================== PHASE 3 =====
hr "PHASE 3  network tokenization: ON for processor A, OFF for processor B"

call POST "${BASE_URL}/account/${PROC_A_MID}/business_profile/${PROC_A_PROFILE}" "04_profile_a_nt_on" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d '{"is_network_tokenization_enabled": true}' >/dev/null
expect 200 "processor A profile: is_network_tokenization_enabled=true"

call POST "${BASE_URL}/account/${PROC_B_MID}/business_profile/${PROC_B_PROFILE}" "05_profile_b_nt_off" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d '{"is_network_tokenization_enabled": false}' >/dev/null
expect 200 "processor B profile: is_network_tokenization_enabled=false"

# provider profile too, so the platform-owned PM can be tokenized
call POST "${BASE_URL}/account/${PROVIDER_MID}/business_profile/${PROVIDER_PROFILE}" "06_profile_provider_nt_on" \
    -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
    -d '{"is_network_tokenization_enabled": true}' >/dev/null
expect 200 "provider profile: is_network_tokenization_enabled=true"

VERIFY_A=$(call GET "${BASE_URL}/account/${PROC_A_MID}/business_profile/${PROC_A_PROFILE}" "07_verify_profile_a" \
    -H "api-key: ${ADMIN_API_KEY}")
VERIFY_B=$(call GET "${BASE_URL}/account/${PROC_B_MID}/business_profile/${PROC_B_PROFILE}" "08_verify_profile_b" \
    -H "api-key: ${ADMIN_API_KEY}")
info "A.is_network_tokenization_enabled = $(echo "$VERIFY_A" | jq -r '.is_network_tokenization_enabled')"
info "B.is_network_tokenization_enabled = $(echo "$VERIFY_B" | jq -r '.is_network_tokenization_enabled')"

# ============================================================== PHASE 4 =====
hr "PHASE 4  connectors (so PML has something to filter against)"

MCA_BODY='{
  "connector_type": "payment_processor",
  "connector_name": "cybersource",
  "connector_account_details": { "auth_type": "SignatureKey", "api_key": "test_key", "key1": "test_merchant", "api_secret": "test_secret" },
  "test_mode": true,
  "disabled": false,
  "business_country": "US",
  "business_label": "default",
  "payment_methods_enabled": [{
    "payment_method": "card",
    "payment_method_types": [
      { "payment_method_type": "credit", "card_networks": ["Visa","Mastercard"], "minimum_amount": 1, "maximum_amount": 68607706, "recurring_enabled": true, "installment_payment_enabled": true },
      { "payment_method_type": "debit", "card_networks": ["Visa","Mastercard"], "minimum_amount": 1, "maximum_amount": 68607706, "recurring_enabled": true, "installment_payment_enabled": true }
    ]
  }]
}'

for M in "$PROVIDER_MID:$PROVIDER_PROFILE:provider" "$PROC_A_MID:$PROC_A_PROFILE:processorA" "$PROC_B_MID:$PROC_B_PROFILE:processorB"; do
    MID="${M%%:*}"; REST="${M#*:}"; PID="${REST%%:*}"; NAME="${REST#*:}"
    call POST "${BASE_URL}/account/${MID}/connectors" "09_mca_${NAME}" \
        -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
        -d "$(echo "$MCA_BODY" | jq -c --arg pid "$PID" '. + {profile_id: $pid}')" >/dev/null
    expect 200 "create MCA for ${NAME}"
done

# ============================================================== PHASE 5 =====
hr "PHASE 5  API keys"

mk_key() {
    local mid="$1" label="$2"
    call POST "${BASE_URL}/api_keys/${mid}" "10_apikey_${label}" \
        -H "api-key: ${ADMIN_API_KEY}" -H 'Content-Type: application/json' \
        -d '{"name":"test key","description":"platform NT suite","expiration":"never"}' \
        | jq -r '.api_key'
}

PROVIDER_KEY=$(mk_key "$PROVIDER_MID" provider);   expect 200 "api key: provider (platform)"
PROC_A_KEY=$(mk_key "$PROC_A_MID" processorA);     expect 200 "api key: processor A"
PROC_B_KEY=$(mk_key "$PROC_B_MID" processorB);     expect 200 "api key: processor B"

info "provider key    : ${PROVIDER_KEY:0:24}..."
info "processor A key : ${PROC_A_KEY:0:24}..."
info "processor B key : ${PROC_B_KEY:0:24}..."

# ============================================================== PHASE 6 =====
hr "PHASE 6  modular OFF — seed platform-owned customer + payment method"

set_modular false
sleep 3

CUSTOMER_REF="cust_plat_${STAMP}"
CUSTOMER=$(call POST "${BASE_URL}/customers" "11_customer_create" \
    -H "api-key: ${PROVIDER_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg cid "$CUSTOMER_REF" '{
          customer_id: $cid,
          name: "John Doe",
          email: "guest@example.com",
          phone: "999999999",
          phone_country_code: "+65",
          description: "platform-owned customer"
        }')")
expect 200 "create customer under PROVIDER key (no connected header)"
CUSTOMER_ID=$(echo "$CUSTOMER" | jq -r '.customer_id')
info "customer_id : ${CUSTOMER_ID}"

PM=$(call POST "${BASE_URL}/payment_methods" "12_pm_create" \
    -H "api-key: ${PROVIDER_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg cid "$CUSTOMER_ID" '{
          payment_method: "card",
          payment_method_type: "credit",
          payment_method_issuer: "Visa",
          customer_id: $cid,
          card: {
            card_number: "4111111145551142",
            card_exp_month: "10",
            card_exp_year: "2031",
            card_holder_name: "John Doe"
          }
        }')")
expect 200 "create payment method under PROVIDER key"
PM_ID=$(echo "$PM" | jq -r '.payment_method_id')
info "payment_method_id : ${PM_ID}"

# ============================================================== PHASE 7 =====
hr "PHASE 7  THE TEST — processor B (NT disabled) reads the platform customer's PML"

info "--- 7a: modular OFF ---"
PML_OFF=$(call GET "${BASE_URL}/customers/${CUSTOMER_ID}/payment_methods" "13_pml_procB_modular_off" \
    -H "api-key: ${PROC_B_KEY}" -H "x-profile-id: ${PROC_B_PROFILE}")
expect 200 "PML via processor B key + profile (modular OFF)"
echo "$PML_OFF" | jq '{count: (.customer_payment_methods | length),
                       methods: [.customer_payment_methods[]? | {payment_token, payment_method, payment_method_id,
                                  requires_cvv, recurring_enabled}]}' 2>/dev/null | sed 's/^/    /'

info "--- 7b: same call with the PROVIDER key (control) ---"
PML_PROV=$(call GET "${BASE_URL}/customers/${CUSTOMER_ID}/payment_methods" "14_pml_provider_modular_off" \
    -H "api-key: ${PROVIDER_KEY}" -H "x-profile-id: ${PROVIDER_PROFILE}")
expect 200 "PML via provider key (control)"
echo "$PML_PROV" | jq '{count: (.customer_payment_methods | length)}' 2>/dev/null | sed 's/^/    /'

info "--- 7c: processor A (NT enabled) for comparison ---"
PML_A=$(call GET "${BASE_URL}/customers/${CUSTOMER_ID}/payment_methods" "15_pml_procA_modular_off" \
    -H "api-key: ${PROC_A_KEY}" -H "x-profile-id: ${PROC_A_PROFILE}")
expect 200 "PML via processor A key + profile"
echo "$PML_A" | jq '{count: (.customer_payment_methods | length)}' 2>/dev/null | sed 's/^/    /'

# ============================================================== PHASE 8 =====
hr "PHASE 8  modular ON — legacy v1 routes must be blocked for API-key traffic"

set_modular true
info "waiting for superposition polling_interval to elapse..."
sleep 12

call GET "${BASE_URL}/customers/${CUSTOMER_ID}/payment_methods" "16_pml_procB_modular_on" \
    -H "api-key: ${PROC_B_KEY}" -H "x-profile-id: ${PROC_B_PROFILE}" >/dev/null
expect 403 "PML via processor B (modular ON) -> AccessForbidden"

call POST "${BASE_URL}/customers" "17_customer_create_modular_on" \
    -H "api-key: ${PROVIDER_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg cid "cust_blocked_${STAMP}" '{customer_id: $cid, name: "Blocked"}')" >/dev/null
expect 403 "POST /customers (modular ON) -> AccessForbidden"

call POST "${BASE_URL}/payment_methods" "18_pm_create_modular_on" \
    -H "api-key: ${PROVIDER_KEY}" -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg cid "$CUSTOMER_ID" '{payment_method: "card", customer_id: $cid,
          card: {card_number: "4111111145551142", card_exp_month: "10", card_exp_year: "2031", card_holder_name: "John Doe"}}')" >/dev/null
expect 403 "POST /payment_methods (modular ON) -> AccessForbidden"

# ============================================================== SUMMARY =====
hr "SUMMARY"
cat <<EOF
  organization_id     : ${ORG_ID}
  provider (platform) : ${PROVIDER_MID}   profile=${PROVIDER_PROFILE}
  processor A (NT on) : ${PROC_A_MID}   profile=${PROC_A_PROFILE}
  processor B (NT off): ${PROC_B_MID}   profile=${PROC_B_PROFILE}
  customer            : ${CUSTOMER_ID}
  payment method      : ${PM_ID}

  responses saved to  : ${OUT_DIR}

  provider key    : ${PROVIDER_KEY}
  processor A key : ${PROC_A_KEY}
  processor B key : ${PROC_B_KEY}

  passed: ${PASS}   failed: ${FAIL}
EOF

# leave modular off so the environment is reusable
set_modular false

[ "$FAIL" -eq 0 ]
