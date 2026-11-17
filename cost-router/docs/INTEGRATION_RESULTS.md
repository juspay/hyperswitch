# Integration results vs a real Hyperswitch (run 2026-11-17)

Environment: Docker Desktop 4.99 / WSL2 (capped 4 GB), image `docker.juspay.io/juspaydotin/hyperswitch-router:standalone`
(router v1.129.0, built 2026-11-01), postgres, redis 7, superposition-demo (required by the server). Dummy connectors
(`stripe_test`, `fauxpay`, `pretendpay`) ARE available in the prebuilt image. Everything below was actually run; the re-runnable
path is `cost-router/scripts/up.sh` then `cost-router/scripts/demo.sh` (end-to-end demo of the cost-router); `scripts/hyperswitch-probe.sh` is the plain Hyperswitch probe used for Part B.

## Setup facts that differ from the repo docs
- The `:standalone` image is older than repo HEAD config: `config/docker_compose.toml` makes it panic on `ecommpay`
  ("Unable to deserialize `ecommpay` as Connector"). Workaround: trimmed config copy in `cost-router/docker/config/` plus
  `cost-router/docker/compose.override.yml` (new files only; tracked files untouched).
- The router panics without superposition ("Failed to initialize superposition client"). `superposition` + `superposition-init` are required.
  `seed_superposition.sh` has CRLF endings and bash-isms; run with bash on a CRLF-stripped copy (`cost-router/docker/seed_superposition.sh`).
- `migration_runner` applies all migrations then fails with "Failed to apply patch: error applying hunk #5" (schema.rs patch, irrelevant to the DB).
- Admin auth: header is `Authorization: admin-api-key=test_admin` (NOT `api-key: test_admin`). `POST /accounts`, `POST /api_keys/{mid}` and `/gsm` take it.
  `POST /account/{mid}/connectors` and business-profile update take the merchant `api-key`.

## Part B
| # | Question | Result |
|---|---|---|
| 1 | `routing: {type: priority, data:[{connector, merchant_connector_id}]}` accepted by POST /payments | PASS. 200. Malformed (`{"connector":"nonexistent"}` only) gives 400 `IR_06 "Invalid straight through routing rules format"`. |
| 2 | Order respected | PASS for first attempt: [fauxpay,stripe_test] -> fauxpay; [stripe_test,fauxpay] -> stripe_test; [pretendpay,fauxpay,stripe_test] -> pretendpay; [fauxpay,pretendpay,stripe_test] -> fauxpay. Without routing the default pick was stripe_test. See "Surprises" for the ineligible-connector behaviour. |
| 3 | `attempts` in create response | FAIL (key absent). `GET /payments/{id}?expand_attempts=true` works and returns `attempts[]`; without the param the key is absent. |
| 4 | Fallback in OUR order | PASS, with extra config (profile auto-retry + GSM rule keyed `sub_flow:"Authorize"`). [fauxpay,pretendpay,stripe_test] -> attempts fauxpay(failure), pretendpay(failure), stripe_test(charged), final `succeeded`; [pretendpay,fauxpay,stripe_test] -> pretendpay, fauxpay, stripe_test; [fauxpay,stripe_test,pretendpay] -> fauxpay, stripe_test (stops on success). |
| 5 | Timeout / ambiguous response | NOT TESTED (dummy connector has no timeout simulation; needs a fake slow connector). |
| 6 | Replayed merchant `payment_id` | PASS (rejected): 400 `{"error":{"type":"invalid_request","message":"The payment with the specified payment_id already exists in our records","code":"HE_01","reason":"PaymentId(\"dup_test_001\") already exists"}}` |

### Surprises (important)
1. **Forced routing is not strict.** If the listed connector is disabled/ineligible/unknown, Hyperswitch silently uses another connector, no error:
   `single` fauxpay with fauxpay disabled -> `succeeded` on stripe_test (200); `priority` [fauxpay(disabled)] only -> stripe_test; priority with a
   bogus `merchant_connector_id` -> stripe_test. Always compare the response `connector` / `attempts` with what was asked for.
2. **List entries without `merchant_connector_id` are ignored.** `[{connector:"fauxpay"}]` -> stripe_test (default), not fauxpay. The id is effectively required.
3. **The list is not a whitelist once retries are on.** `single` fauxpay with card 4000000000009995: attempt 1 fauxpay fails, attempt 2 stripe_test (not in the request) succeeds.
   Priority [fauxpay,pretendpay] only: 3 attempts, the third on stripe_test (not in the list). So "single = no failover" is FALSE when profile auto-retry is on. The only
   way found to stop failover is turning `is_auto_retries_enabled` off on the profile (then [fauxpay,stripe_test] -> one attempt, `failed` on fauxpay). No per-request switch found.
4. `attempts[]` entries have `attempt_id`, `status` (`failure`, `charged`), `connector`, `error_code`, ... but **no `merchant_connector_id`**.
   Array order is not guaranteed (saw _3,_2,_1 and _1,_2,_3); sort by the `_N` suffix. Expanded-attempt `error_message` was null while the payment-level one is set.
5. Auto-retry needs: profile `{"is_auto_retries_enabled":true,"max_auto_retries_enabled":N}` (N=2 gave up to 3 attempts) AND a GSM rule with `decision:retry`.
   GSM lookup key is `flow:"Payment"`, `sub_flow:"Authorize"` (capital A; `authorize_flow` silently misses, so no retry), plus `code` and `message` exactly as the connector returned.
   Dummy connector error for card 4000000000009995 on fauxpay/pretendpay: code `DC_08`, message `Payment declined: Internal Server Error from Connector, Please try again later`
   (stripe_test succeeds on that card, a deterministic "first fails, last succeeds" fixture). Card 4242424242424242 succeeds everywhere; 4000000000000002 declines everywhere.
6. Without a matching GSM rule a failed payment simply ends `failed` on the first connector (unified_code UE_9000), no retry.

### Key commands (full set in scripts/hyperswitch-probe.sh)
```
curl -s -X POST localhost:8080/accounts -H 'Authorization: admin-api-key=test_admin' -H 'Content-Type: application/json' -d '{"merchant_id":"merchant_cr1","merchant_name":"CostRouter Test","primary_business_details":[{"country":"US","business":"default"}]}'
curl -s -X POST localhost:8080/api_keys/merchant_cr1 -H 'Authorization: admin-api-key=test_admin' -H 'Content-Type: application/json' -d '{"name":"cr","description":"d","expiration":"never"}'
curl -s -X POST localhost:8080/account/merchant_cr1/connectors -H "api-key: $KEY" -H 'Content-Type: application/json' -d '{"connector_type":"payment_processor","connector_name":"fauxpay","connector_account_details":{"auth_type":"HeaderKey","api_key":"dummy"},"test_mode":true,"disabled":false,"payment_methods_enabled":[...card credit, see demo.sh...]}'
curl -s -X POST localhost:8080/account/merchant_cr1/business_profile/$PROFILE -H "api-key: $KEY" -H 'Content-Type: application/json' -d '{"is_auto_retries_enabled":true,"max_auto_retries_enabled":2}'
curl -s -X POST localhost:8080/gsm -H 'Authorization: admin-api-key=test_admin' -H 'Content-Type: application/json' -d '{"connector":"fauxpay","flow":"Payment","sub_flow":"Authorize","code":"DC_08","message":"Payment declined: Internal Server Error from Connector, Please try again later","status":"failure","decision":"retry","step_up_possible":false,"clear_pan_possible":false,"feature":"retry","feature_data":{"retry":{"step_up_possible":false,"clear_pan_possible":false,"alternate_network_possible":false,"decision":"retry"}}}'
curl -s "localhost:8080/payments/$ID?expand_attempts=true" -H "api-key: $KEY"
```
Failover result (real, trimmed): `status succeeded, connector stripe_test, attempt_count 3`; attempts `_1 failure fauxpay DC_08`, `_2 failure pretendpay DC_08`, `_3 charged stripe_test`.

## Part C (cost-router on :3100 against the live stack)
Config: `cost-router/config.json` (candidates stripe_test/fauxpay/pretendpay with real mca ids, USD fee table; the API key only via `HYPERSWITCH_API_KEY`).
`cost-router/.gitignore` did NOT contain `config.json` (only node_modules, dist); I appended `config.json` (the file holds no secret but is environment specific).
Run: `HYPERSWITCH_API_KEY=... COST_ROUTER_CONFIG=config.json node dist/main.js` (after `npm run build`).

| Check | Result |
|---|---|
| build + start | PASS |
| POST /payments with Idempotency-Key `crpay_001`: priority override sent, ordered [fauxpay, pretendpay, stripe_test] (cheapest first); Hyperswitch handled it on fauxpay (the default would have been stripe_test) | PASS |
| response contains `routing` trace (decision.ordered, per-candidate score/fee/sHat/observed, summary) plus `payment` | PASS |
| `GET /payments/crpay_003/routing-trace` 200 with stored decision; unknown id 404 | PASS |
| chosen order == actual attempt sequence | PASS: crpay_003 decision [pretendpay, fauxpay, stripe_test]; Hyperswitch attempts `_1 pretendpay failure, _2 fauxpay failure, _3 stripe_test charged` |
| stats updated from real attempts | PASS: after crpay_002 (card 9995: fauxpay fail, pretendpay fail, stripe_test ok) the crpay_003 trace showed fauxpay observed {success 1, technical_failure 1}, pretendpay {technical_failure 1}, stripe_test {success 1}; the ranking changed from fauxpay-first to pretendpay-first |
| replay of same Idempotency-Key and body | PASS: `replayed:true`, stored payment returned, not forwarded again |
| same key, different body | PASS: 409 Conflict |

No cost-router source was changed. Note: pretendpay still ranked first after a technical failure because shrinkage (min_sample_size 5) keeps thin data close to the prior and its fee is below stripe_test's.
