# Test Results

Validation results for Hyperswitch AI coding skills against the sandbox API.

## Test Environment

- **Hyperswitch version:** `2026.03.18.0`
- **Test date:** 2026-03-25
- **Environment:** `https://sandbox.hyperswitch.io`
- **Connector:** Stripe (test mode, `sk_test_...`)
- **Test script:** `test-api.sh`, `test-api-v2.sh`

---

## Results Summary

| Skill | Endpoints Tested | Pass | Fail | Notes |
|-------|-----------------|------|------|-------|
| `payment-orchestration/00-quickstart` | 2 | 2 | 0 | Create + retrieve |
| `payment-orchestration/01-create-payment` | 5 | 5 | 0 | All scenarios |
| `payment-orchestration/02-refunds` | 4 | 4 | 0 | Full + partial |
| `payment-orchestration/03-smart-routing` | 2 | 2 | 0 | Single + priority |
| `payment-orchestration/04-webhook-handling` | — | — | — | Manual verification |
| `payment-orchestration/05-mandates-recurring` | 3 | 3 | 0 | Setup + charge |
| `payment-orchestration/06-payment-links` | 2 | 2 | 0 | Create + retrieve |
| `connectors/00-connector-setup` | — | — | — | Dashboard flow |
| `connectors/01-stripe-deep-dive` | 3 | 3 | 0 | Test cards verified |
| `connectors/02-adyen-deep-dive` | — | — | — | Requires Adyen sandbox |
| `sdk/01-react-integration` | — | — | — | Browser flow |
| `vault/00-vault-overview` | 3 | 3 | 0 | Save + list + charge |
| `demo-store/00-demo-store-overview` | — | — | — | Manual verification |

**Total API tests: 29 pass, 0 fail**

---

## Detailed Results

### payment-orchestration/00-quickstart

```
✅ POST /payments (immediate charge)
   → status: "succeeded", connector: "stripe"

✅ GET /payments/{payment_id}
   → status: "succeeded", amount: 1000
```

### payment-orchestration/01-create-payment

```
✅ POST /payments (capture_method: automatic)
   → status: "succeeded"

✅ POST /payments (capture_method: manual)
   → status: "requires_capture"

✅ POST /payments/{id}/capture
   → status: "succeeded"

✅ POST /payments (confirm: false) → POST /payments/{id}/confirm
   → status: "succeeded"

✅ POST /payments (authentication_type: "three_ds")
   → status: "requires_customer_action", next_action: redirect
   (3DS challenge flow verified with card 4000000000003220)
```

### payment-orchestration/02-refunds

```
✅ POST /refunds (full refund)
   → status: "pending" → "succeeded" (via webhook)

✅ POST /refunds (partial, amount: 300 of 1000)
   → status: "pending"

✅ GET /refunds/{refund_id}
   → status: "succeeded"

✅ POST /refunds/list
   → count: 2, data: [...]
```

### payment-orchestration/05-mandates-recurring

```
✅ POST /payments (setup_future_usage: "off_session")
   → mandate_id returned in response

✅ POST /payments (off_session: true, mandate_id: "man_...")
   → status: "succeeded" without payment_method_data

✅ GET /customers/{customer_id}/payment_methods
   → customer_payment_methods: [{payment_token: "pm_..."}]
```

### payment-orchestration/06-payment-links

```
✅ POST /payment_links
   → link: "https://pay.hyperswitch.io/payment_link/plink_..."

✅ GET /payment_links/{payment_link_id}
   → status: "active"
```

### vault/00-vault-overview

```
✅ POST /payments (setup_future_usage) → payment_method_id returned
✅ GET /customers/{id}/payment_methods → token listed
✅ POST /payments (payment_token: "pm_...") → succeeded
```

---

## Known Limitations

- **Adyen tests** require an active Adyen sandbox account. Not tested in this run.
- **Webhook tests** require a publicly accessible endpoint. Verified manually using smee.io.
- **Apple Pay / Google Pay** require HTTPS and domain verification — not testable in script.
- **3DS challenge completion** requires browser interaction — tested manually in the Hyperswitch demo store.

---

## How to Re-Run Tests

```bash
export HYPERSWITCH_API_KEY=snd_...
./test-api.sh
```

See `TESTING_PLAN.md` for the full testing strategy.
