---
name: hyperswitch-create-payment
description: Use this skill when the user wants to "create a payment", "charge a customer", "initiate a transaction", "auth-only payment", "authorize and capture", "create payment with 3DS", "accept a wallet payment", "PayPal payment", "Apple Pay integration", "build a PaymentsRequest", "what fields does POST /payments accept", or needs to understand all payment creation patterns in Hyperswitch.
version: 1.0.0
tags: [hyperswitch, payments, create, authorization, capture, 3DS, wallets]
---

# Create a Payment

## Overview

`POST /payments` is the central endpoint in Hyperswitch. It handles one-time charges, auth-only flows, 3DS authentication, wallet redirects, and more — all through a single, consistent interface. This skill covers every major creation pattern.

## Prerequisites

- Hyperswitch API key (`api-key` header)
- At least one connector configured in the dashboard
- For redirect-based methods (wallets, 3DS): a `return_url` endpoint on your server

---

## Core Request Structure

```json
POST https://sandbox.hyperswitch.io/payments
{
  "amount": 1000,
  "currency": "USD",
  "confirm": true,
  "capture_method": "automatic",
  "payment_method": "card",
  "payment_method_data": { ... },
  "customer_id": "cus_optional",
  "return_url": "https://yourapp.com/payment/complete",
  "description": "Order #1234",
  "metadata": { "order_id": "ORD-1234" }
}
```

**Key fields:**

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `amount` | integer | required | Smallest currency unit ($10.00 = `1000`) |
| `currency` | string | required | ISO 4217 (`USD`, `EUR`, `GBP`, `INR`, …) |
| `confirm` | boolean | `false` | `true` = authorize immediately |
| `capture_method` | enum | `automatic` | `automatic` or `manual` |
| `payment_method` | enum | required if confirming | `card`, `wallet`, `bank_redirect`, `pay_later`, … |
| `payment_method_data` | object | required if confirming | Nested under `payment_method` type |
| `return_url` | string | conditional | Required for 3DS and wallet redirects |
| `authentication_type` | enum | `no_three_ds` | `three_ds` to force 3DS |

---

## Scenario 1: Immediate Charge (Auth + Capture)

Authorize and capture in one step — the default for most e-commerce:

```json
POST /payments
{
  "amount": 2999,
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
  "customer_id": "cus_abc123",
  "description": "Pro subscription — March 2024",
  "metadata": { "plan": "pro", "billing_cycle": "2024-03" }
}
```

**Response `status`:** `succeeded`

---

## Scenario 2: Authorization Only (Capture Later)

Hold funds on the card; capture after fulfillment (e.g., hotels, marketplaces):

```json
POST /payments
{
  "amount": 15000,
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
  }
}
```

**Response `status`:** `requires_capture`

Capture later with:
```bash
POST /payments/{payment_id}/capture
{ "amount_to_capture": 15000 }
```

---

## Scenario 3: Two-Step — Create then Confirm

Create the payment object first, attach payment method when ready:

```json
// Step 1: Create (no card yet)
POST /payments
{
  "amount": 5000,
  "currency": "EUR",
  "confirm": false,
  "customer_id": "cus_abc123"
}
// Returns payment_id, status: "requires_payment_method"

// Step 2: Confirm with card
POST /payments/{payment_id}/confirm
{
  "payment_method": "card",
  "payment_method_data": {
    "card": { "card_number": "4242424242424242", "card_exp_month": "03", "card_exp_year": "2030", "card_cvc": "737" }
  },
  "return_url": "https://yourapp.com/complete"
}
```

---

## Scenario 4: 3D Secure (SCA / PSD2)

Force 3DS authentication — required for EU payments:

```json
POST /payments
{
  "amount": 5000,
  "currency": "EUR",
  "confirm": true,
  "capture_method": "automatic",
  "authentication_type": "three_ds",
  "return_url": "https://yourapp.com/payment/3ds/complete",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4000000000003220",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  }
}
```

**Response when challenge required:**
```json
{
  "status": "requires_customer_action",
  "next_action": {
    "type": "redirect_to_url",
    "redirect_to_url": { "url": "https://acs.bank.com/3ds/...", "return_url": "..." }
  }
}
```

→ Redirect the user to `next_action.redirect_to_url.url`. After they complete the challenge, they return to your `return_url` and you call `POST /payments/{id}/complete_authorize`.

---

## Scenario 5: PayPal (Wallet Redirect)

```json
POST /payments
{
  "amount": 3500,
  "currency": "USD",
  "confirm": true,
  "return_url": "https://yourapp.com/payment/complete",
  "payment_method": "wallet",
  "payment_method_data": {
    "wallet": {
      "paypal_redirect": {}
    }
  }
}
```

→ Redirect user to `next_action.redirect_to_url.url`. On return, verify status via `GET /payments/{id}`.

---

## Scenario 6: With Customer and Billing Address

```json
POST /payments
{
  "amount": 7999,
  "currency": "GBP",
  "confirm": true,
  "capture_method": "automatic",
  "customer_id": "cus_abc123",
  "email": "customer@example.com",
  "name": "Jane Smith",
  "phone": "+447911123456",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737",
      "card_holder_name": "Jane Smith"
    }
  },
  "billing": {
    "address": {
      "line1": "123 High Street",
      "city": "London",
      "zip": "SW1A 1AA",
      "country": "GB"
    }
  },
  "description": "Annual membership",
  "statement_descriptor_name": "YOURCO MEMB",
  "metadata": { "membership_id": "MEM-2024-001" }
}
```

---

## Payment Method Types

| `payment_method` | Sub-types (in `payment_method_data`) |
|-----------------|--------------------------------------|
| `card` | `card: { card_number, card_exp_month, card_exp_year, card_cvc }` |
| `wallet` | `paypal_redirect`, `apple_pay`, `google_pay`, `samsung_pay` |
| `bank_redirect` | `ideal`, `sofort`, `giropay`, `eps`, `bancontact`, `blik` |
| `pay_later` | `klarna`, `affirm`, `afterpay_clearpay` |
| `bank_debit` | `ach`, `sepa`, `bacs` |
| `crypto` | `crypto_currency` |

---

## Status Transitions

```
requires_payment_method
        ↓ (confirm)
requires_confirmation
        ↓
    processing
        ↓
  ┌─────┴──────┐
succeeded  requires_capture  requires_customer_action
                ↓ (capture)         ↓ (complete_authorize)
            succeeded            succeeded
```

---

## Error Handling

| HTTP Status | `error.code` | Meaning |
|-------------|-------------|---------|
| `422` | `IR_01` | Missing required field |
| `422` | `IR_06` | Invalid enum value for `payment_method` |
| `402` | `HE_01` | Connector declined — check `error.message` |
| `401` | `IR_16` | Invalid API key |
| `404` | `HE_02` | Payment not found |

---

## Production Tips

- Always pass an `Idempotency-Key` header (UUID v4) to safely retry without double-charging.
- `metadata` is server-side only — never expose internal IDs (order IDs, user IDs) to the client response.
- `statement_descriptor_name` (max 22 chars) is what appears on the cardholder's bank statement — make it recognizable to reduce chargebacks.
- For SCA compliance in the EU, set `authentication_type: "three_ds"` even if your connector can handle it natively — Hyperswitch will negotiate frictionless when possible.
- `amount` must equal `amount_to_capture` unless you intend a partial capture — over-capturing returns a 422.
