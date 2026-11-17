#!/usr/bin/env bash
# Re-runnable end-to-end demo against a running stack (run up.sh first). Creates a fresh merchant per run.
set -u
H=${H:-http://localhost:8080}; ADMIN="Authorization: admin-api-key=test_admin"; J="Content-Type: application/json"
R=$(date +%s); MID="merchant_demo_$R"
jf(){ python -c "import sys,json; d=json.load(sys.stdin); print(d$1)"; }
echo "== health"; curl -s $H/health; echo
echo "== merchant"; M=$(curl -s -X POST $H/accounts -H "$ADMIN" -H "$J" -d '{"merchant_id":"'$MID'","merchant_name":"Demo","primary_business_details":[{"country":"US","business":"default"}]}')
PROFILE=$(echo "$M" | jf "['default_profile']"); echo "profile=$PROFILE"
echo "== api key"; KEY=$(curl -s -X POST $H/api_keys/$MID -H "$ADMIN" -H "$J" -d '{"name":"demo","description":"d","expiration":"never"}' | jf "['api_key']"); echo "key=${KEY:0:12}..."
PM='[{"payment_method":"card","payment_method_types":[{"payment_method_type":"credit","card_networks":["Visa","Mastercard"],"minimum_amount":0,"maximum_amount":99999999,"recurring_enabled":true,"installment_payment_enabled":true}]}]'
mca(){ curl -s -X POST $H/account/$MID/connectors -H "api-key: $KEY" -H "$J" -d '{"connector_type":"payment_processor","connector_name":"'$1'","connector_account_details":{"auth_type":"HeaderKey","api_key":"dummy"},"test_mode":true,"disabled":false,"payment_methods_enabled":'"$PM"'}' | jf "['merchant_connector_id']"; }
echo "== connectors"; S=$(mca stripe_test); F=$(mca fauxpay); P=$(mca pretendpay); echo "stripe_test=$S fauxpay=$F pretendpay=$P"
echo "== auto-retry on profile (max 2)"; curl -s -X POST $H/account/$MID/business_profile/$PROFILE -H "api-key: $KEY" -H "$J" -d '{"is_auto_retries_enabled":true,"max_auto_retries_enabled":2}' | jf "['max_auto_retries_enabled']"
echo "== GSM retry rules (global; duplicates on rerun return an error, ignore)"
for c in fauxpay pretendpay; do curl -s -o /dev/null -w "gsm $c: %{http_code}\n" -X POST $H/gsm -H "$ADMIN" -H "$J" -d '{"connector":"'$c'","flow":"Payment","sub_flow":"Authorize","code":"DC_08","message":"Payment declined: Internal Server Error from Connector, Please try again later","status":"failure","decision":"retry","step_up_possible":false,"clear_pan_possible":false,"feature":"retry","feature_data":{"retry":{"step_up_possible":false,"clear_pan_possible":false,"alternate_network_possible":false,"decision":"retry"}}}'; done
pay(){ curl -s -X POST $H/payments -H "api-key: $KEY" -H "$J" -d '{"amount":6540,"currency":"USD","confirm":true,"capture_method":"automatic","authentication_type":"no_three_ds","profile_id":"'$PROFILE'","payment_method":"card","payment_method_type":"credit","payment_method_data":{"card":{"card_number":"'${2:-4242424242424242}'","card_exp_month":"12","card_exp_year":"2030","card_holder_name":"T","card_cvc":"123"}},"billing":{"address":{"line1":"1 Main","city":"SF","state":"CA","zip":"94000","country":"US","first_name":"A","last_name":"B"}}'"$1"'}'; }
sm(){ python -c "import sys,json; d=json.load(sys.stdin); print({k:d.get(k) for k in ['payment_id','status','connector','attempt_count','error_code'] if k in d} if 'error' not in d else d)"; }
att(){ curl -s "$H/payments/$1?expand_attempts=true" -H "api-key: $KEY" | python -c "import sys,json; d=json.load(sys.stdin); [print('   ',a['attempt_id'],a['status'],a['connector'],a['error_code']) for a in sorted(d['attempts'],key=lambda a:a['attempt_id'])]"; }
rt(){ echo '"routing":{"type":"priority","data":['"$1"']}'; }
c(){ echo '{"connector":"'$1'","merchant_connector_id":"'$2'"}'; }
echo "== plain payment (no routing)"; pay "" | sm
echo "== priority [fauxpay, stripe_test] -> expect fauxpay"; pay ",$(rt "$(c fauxpay $F),$(c stripe_test $S)")" | sm
echo "== priority [stripe_test, fauxpay] -> expect stripe_test"; pay ",$(rt "$(c stripe_test $S),$(c fauxpay $F)")" | sm
echo "== single fauxpay"; pay ',"routing":{"type":"single","data":{"connector":"fauxpay","merchant_connector_id":"'$F'"}}' | sm
echo "== failover [fauxpay, pretendpay, stripe_test], card 4000000000009995 (fails on all but stripe_test)"
r=$(pay ",$(rt "$(c fauxpay $F),$(c pretendpay $P),$(c stripe_test $S)")" 4000000000009995); echo "$r" | sm; att $(echo "$r" | jf "['payment_id']")
echo "== replayed payment_id"; pay ',"payment_id":"dup_'$R'"' | sm; pay ',"payment_id":"dup_'$R'"' | sm
echo "== cost-router (needs it running on :3100 with a config for THIS merchant; see INTEGRATION_RESULTS.md)"
echo "export HYPERSWITCH_API_KEY=$KEY ; mca ids: stripe_test=$S fauxpay=$F pretendpay=$P profile=$PROFILE"
