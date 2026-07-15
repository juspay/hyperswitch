#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# tsys_transit — payment_method_id MIT via StoredCardForNetworkTransactionId
# ---------------------------------------------------------------------------
# Cert-style curl test cases for the new locker-sourced (payment_method_id) MIT
# flow. Exercises HS (/payments) -> UCS (gRPC) -> TSYS end to end.
#
#   CIT : save a card off_session; the connector stores the network transaction
#         id in BOTH connector_mandate_id and network_transaction_id.
#   MIT : replay with recurring_details.type = payment_method_id. Because
#         tsys_transit is in pmid_mit_supported_connectors, HS pulls the card
#         from the locker and sends it as StoredCardForNetworkTransactionId
#         (card + NTI) instead of the connector-mandate path.
#
# Expected: CIT -> succeeded (pm_id returned); MIT -> succeeded (TSYS A0000),
#           HS log shows "using card with network_transaction_id for MIT flow".
#
# Usage:  BASE_URL=http://localhost:8080 API_KEY=dev_xxx MCA=mca_xxx \
#         PROFILE=pro_xxx ./pmid_mit_storedcard_test.sh
# ---------------------------------------------------------------------------
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_KEY="${API_KEY:?set API_KEY}"
MCA="${MCA:?set MCA (merchant_connector_id)}"
PROFILE="${PROFILE:?set PROFILE (profile_id)}"
CARD_NUMBER="${CARD_NUMBER:-4012000098765439}"
CUST="pmid_mit_$(date +%s)"

card_json='"payment_method":"card","payment_method_type":"credit","payment_method_data":{"card":{"card_number":"'"$CARD_NUMBER"'","card_exp_month":"12","card_exp_year":"2028","card_cvc":"999","card_holder_name":"CERT MIT","card_network":"Visa"}},"billing":{"address":{"line1":"8320 Test St","zip":"85284","city":"Tempe","state":"AZ","country":"US"}}'
route_json='"routing":{"type":"single","data":{"connector":"tsys_transit","merchant_connector_id":"'"$MCA"'"}},"profile_id":"'"$PROFILE"'"'

jqget() { python3 -c "import sys,json;print(json.load(sys.stdin).get('$1') or '')"; }

echo "############ CIT — save card (off_session) ############"
cit=$(curl -s --max-time 60 "$BASE_URL/payments" \
  -H 'Content-Type: application/json' -H "api-key: $API_KEY" \
  --data '{"amount":347,"currency":"USD","confirm":true,"capture_method":"automatic","payment_channel":"telephone_order","customer_id":"'"$CUST"'","setup_future_usage":"off_session","customer_acceptance":{"acceptance_type":"offline"},'"$route_json"','"$card_json"'}')
cit_status=$(echo "$cit" | jqget status)
pm_id=$(echo "$cit" | jqget payment_method_id)
echo "  status=$cit_status  payment_method_id=$pm_id"
[ "$cit_status" = "succeeded" ] && [ -n "$pm_id" ] || { echo "  ✗ CIT FAILED"; echo "$cit"; exit 1; }
echo "  ✓ CIT ok"

echo "############ MIT — recurring via payment_method_id ############"
mit=$(curl -s --max-time 60 "$BASE_URL/payments" \
  -H 'Content-Type: application/json' -H "api-key: $API_KEY" \
  --data '{"amount":529,"currency":"USD","confirm":true,"off_session":true,"capture_method":"automatic","payment_channel":"telephone_order","customer_id":"'"$CUST"'","recurring_details":{"type":"payment_method_id","data":"'"$pm_id"'"},'"$route_json"'}')
mit_status=$(echo "$mit" | jqget status)
mit_received=$(echo "$mit" | jqget amount_received)
echo "  status=$mit_status  amount_received=$mit_received"
[ "$mit_status" = "succeeded" ] || { echo "  ✗ MIT FAILED"; echo "$mit"; exit 1; }
echo "  ✓ MIT ok — StoredCardForNetworkTransactionId flow"

echo ""
echo "PASS ✅  CIT=$cit_status  MIT=$mit_status (pm_id=$pm_id)"
