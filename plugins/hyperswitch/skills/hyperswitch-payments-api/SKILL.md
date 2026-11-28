---
name: hyperswitch-payments-api
description: >-
  Call the Hyperswitch v1 REST API end to end: admin vs merchant
  authentication, create a merchant account, API key and connector
  account, then create, confirm, capture, retrieve and refund payments.
  Use when testing payment flows against a Hyperswitch server or writing
  backend code that talks to it.
---

# Hyperswitch payments API (v1)

References: `api-reference/v1/openapi_spec_v1.json`,
`api-reference/essentials/authentication.mdx`, `docs/try_local_system.md`.

## Auth

| Key | Header | Use |
|-----|--------|-----|
| Admin key (`admin_api_key` in config) | `Authorization: admin-api-key=<key>` | Merchant accounts, connector accounts, API keys |
| Merchant API key | `api-key: <key>` | Payments, refunds, customers (server-side only) |
| Publishable key `pk_...` | client SDK | Safe for browsers/apps |

Admin credentials in the `api-key` header are no longer accepted.

```bash
export HS=http://localhost:8080 ADMIN=test_admin
```

## Bootstrap a merchant

```bash
curl -s $HS/accounts -H "Authorization: admin-api-key=$ADMIN" -H 'Content-Type: application/json' \
  -d '{"merchant_id":"merchant_demo"}'                       # note publishable_key

curl -s $HS/api_keys/merchant_demo -H "Authorization: admin-api-key=$ADMIN" -H 'Content-Type: application/json' \
  -d '{"name":"local","expiration":"never"}'                 # plaintext key is shown once

curl -s $HS/account/merchant_demo/connectors -H "Authorization: admin-api-key=$ADMIN" -H 'Content-Type: application/json' \
  -d '{"connector_type":"payment_processor","connector_name":"stripe",
       "connector_account_details":{"auth_type":"HeaderKey","api_key":"<stripe test secret>"},
       "test_mode":true}'
```

`connector_account_details.auth_type` depends on the connector (e.g.
Adyen uses `BodyKey` with `api_key` + `key1`). Use sandbox credentials
only, and never print real keys back to the user.

## Payments

```bash
export KEY=<merchant api key>
curl -s $HS/payments -H "api-key: $KEY" -H 'Content-Type: application/json' -d '{
  "amount": 6540, "currency": "USD", "confirm": true,
  "payment_method": "card", "authentication_type": "no_three_ds",
  "payment_method_data": {"card": {"card_number": "4242424242424242",
    "card_exp_month": "10", "card_exp_year": "30", "card_cvc": "123", "card_holder_name": "Test"}}
}'
curl -s $HS/payments/<payment_id> -H "api-key: $KEY"
```

- `amount` is in the minor unit (6540 = $65.40).
- Status `requires_confirmation`: call `POST /payments/{id}/confirm`.
- `"capture_method": "manual"` then `POST /payments/{id}/capture`.
- `POST /payments/{id}/cancel` voids; `GET /payments/list` lists.

## Refunds

```bash
curl -s $HS/refunds -H "api-key: $KEY" -H 'Content-Type: application/json' \
  -d '{"payment_id":"<payment_id>","amount":1000,"reason":"test"}'
curl -s $HS/refunds/<refund_id> -H "api-key: $KEY"
```

Only run payment or refund calls against live (non-test) connectors with
the user's explicit go-ahead.
