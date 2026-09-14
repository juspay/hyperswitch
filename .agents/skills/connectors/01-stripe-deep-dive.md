---
name: hyperswitch-stripe-connector
description: Use this skill when the user asks about "Stripe connector in Hyperswitch", "Stripe API key Hyperswitch", "Stripe webhook forwarding", "Stripe 3DS in Hyperswitch", "Stripe test cards", "Stripe payment methods via Hyperswitch", "Stripe radar", "Stripe Connect in Hyperswitch", "configure Stripe", or encounters Stripe-specific errors when using Hyperswitch.
version: 1.0.0
tags: [hyperswitch, stripe, connector, integration]
---

# Stripe Connector Deep Dive

## Overview

Stripe is typically the first connector developers configure in Hyperswitch due to its excellent sandbox tooling and extensive payment method support. This guide covers everything Stripe-specific: credential setup, supported methods, webhook forwarding, 3DS behavior, known quirks, and debugging tips.

## Prerequisites

- Stripe account ([dashboard.stripe.com](https://dashboard.stripe.com))
- Hyperswitch sandbox account with admin access

---

## Step 1: Retrieve Stripe Credentials

In the Stripe Dashboard:

1. **Developers → API Keys**
2. Copy **Secret Key** (`sk_test_...` for test mode)
3. Keep the **Publishable Key** (`pk_test_...`) handy — needed if you use Stripe.js directly

> **Never use live keys** (`sk_live_...`) in Hyperswitch sandbox. Always use `sk_test_...` in test environments.

---

## Step 2: Configure in Hyperswitch

In **Hyperswitch Dashboard → Connectors → Stripe**:

| Field | Value |
|-------|-------|
| API Key | `sk_test_...` (your Stripe secret key) |
| Webhook Secret | `whsec_...` (from Stripe webhook endpoint — see Step 3) |
| Mode | Test |

---

## Step 3: Set Up Stripe Webhooks

Stripe sends raw events directly to Hyperswitch (for dispute notifications, async payment updates). Configure this forwarding:

1. **Stripe Dashboard → Developers → Webhooks → Add Endpoint**
2. Endpoint URL: `https://sandbox.hyperswitch.io/webhooks/{your_merchant_id}/stripe`
3. Events to send: `charge.dispute.created`, `charge.refund.updated`, `payment_intent.payment_failed`
4. Copy the **Signing Secret** (`whsec_...`) and enter it in Hyperswitch

> This is separate from your Hyperswitch-to-your-server webhooks. This is Stripe-to-Hyperswitch.

---

## Supported Payment Methods via Stripe

| Payment Method | Hyperswitch `payment_method` | Notes |
|---------------|------------------------------|-------|
| Credit/Debit Card | `card` | Visa, Mastercard, Amex, Discover |
| Apple Pay | `wallet` → `apple_pay` | Requires HTTPS + domain verification |
| Google Pay | `wallet` → `google_pay` | Works in Chrome/Android |
| PayPal | `wallet` → `paypal_redirect` | Redirect flow |
| iDEAL | `bank_redirect` → `ideal` | Netherlands |
| SOFORT | `bank_redirect` → `sofort` | Germany, Austria |
| Bancontact | `bank_redirect` → `bancontact` | Belgium |
| SEPA Direct Debit | `bank_debit` → `sepa` | Mandates required |
| Klarna | `pay_later` → `klarna` | Requires Stripe Klarna activation |
| Afterpay/Clearpay | `pay_later` → `afterpay_clearpay` | |

---

## Stripe Test Cards

| Card Number | Network | Scenario |
|-------------|---------|----------|
| `4242424242424242` | Visa | Success |
| `4000056655665556` | Visa (debit) | Success |
| `5555555555554444` | Mastercard | Success |
| `378282246310005` | Amex | Success |
| `4000000000000002` | Visa | Declined (generic) |
| `4000000000009995` | Visa | Insufficient funds |
| `4000000000000069` | Visa | Expired card |
| `4000000000000127` | Visa | Incorrect CVC |
| `4000000000003220` | Visa | 3DS challenge required |
| `4000000000003063` | Visa | 3DS frictionless |
| `4000000000003089` | Visa | 3DS — authenticated but payment failed |

All test cards: expiry = any future date, CVC = any 3 digits (4 for Amex).

---

## 3DS Behavior with Stripe

When `authentication_type: "three_ds"` is set, Hyperswitch passes this to Stripe's PaymentIntents API with `confirm: true` and `return_url`. Stripe's 3DS handling:

- **Frictionless**: Stripe's ACS authenticates in the background. Payment proceeds to `succeeded` without a redirect.
- **Challenge**: Customer is redirected to the bank's ACS. Your `return_url` receives the customer back.

After the redirect:
```bash
POST /payments/{payment_id}/complete_authorize
# No body needed — Hyperswitch reads the 3DS result from the redirect params
```

---

## Stripe-Specific Error Codes

| Stripe Code | Hyperswitch Mapping | Meaning |
|------------|---------------------|---------|
| `card_declined` | `HE_01` | Generic decline |
| `insufficient_funds` | `HE_01` | Not enough balance |
| `expired_card` | `HE_01` | Card past expiry |
| `incorrect_cvc` | `HE_01` | Wrong security code |
| `processing_error` | `HE_03` | Stripe processing issue — safe to retry |
| `authentication_required` | `HE_01` | 3DS required but not initiated |

Access the raw Stripe error via `payment.error_code` and `payment.error_message` in the Hyperswitch response.

---

## Known Stripe Quirks in Hyperswitch

1. **Capture window**: Stripe authorizations expire after **7 days**. If you use `capture_method: manual`, capture within this window or the authorization is voided automatically.

2. **Idempotency keys**: Stripe enforces idempotency key uniqueness for 24 hours. Hyperswitch generates these automatically — do not override unless you fully understand the implications.

3. **Refund timing**: Stripe sandbox refunds appear as `succeeded` immediately. Production refunds take 5–10 business days to appear on customer statements.

4. **Statement descriptor**: `statement_descriptor_name` maps to Stripe's `statement_descriptor` (max 22 chars, no `<>\"'` characters).

5. **Webhook forwarding for disputes**: Stripe sends dispute webhooks directly to Hyperswitch (Step 3 above). If this is not configured, `dispute.opened` events will not fire in Hyperswitch.

---

## Debugging Stripe Payments

When a Stripe payment fails, cross-reference in both dashboards:

1. Hyperswitch Dashboard → Payments → find `connector_transaction_id`
2. Stripe Dashboard → Payments → search by the Stripe `payment_intent_id` (that's your `connector_transaction_id`)
3. Stripe shows the full decline reason, 3DS outcome, and any Radar rules that fired

---

## Production Tips

- Stripe Radar (fraud rules) runs on all payments. Review your Radar rules in Stripe Dashboard if you see unexpectedly high decline rates.
- For 3D Secure, Stripe handles `authentication_type: "three_ds"` by requesting a 3DS exemption where possible — you typically don't need to force 3DS for every EU payment if using Stripe.
- Rotate API keys via Stripe Dashboard → Developers → API Keys → Roll. Update in Hyperswitch immediately after. Do this on a regular schedule or after any team member with access departs.
- Enable **Stripe Radar for Fraud Teams** (paid feature) if you process >$1M/month — the default Radar rules are coarse.
