---
name: hyperswitch-mandates-recurring
description: Use this skill when the user asks about "recurring payments", "subscriptions", "mandates", "save card for later", "setup_future_usage", "off-session payments", "MIT transactions", "merchant-initiated transactions", "charge a saved card", "customer_acceptance", "repeat billing", "charge without customer present", or needs to implement recurring billing in Hyperswitch.
version: 1.0.0
tags: [hyperswitch, mandates, recurring, subscriptions, MIT, setup_future_usage]
---

# Mandates & Recurring Payments

## Overview

Recurring billing requires two phases: (1) obtaining customer consent and saving the payment method during an on-session transaction, and (2) charging that saved method later without the customer present (off-session / MIT). Hyperswitch handles this via `setup_future_usage`, `mandate_data`, and `mandate_id` / `payment_token` on subsequent charges.

## Prerequisites

- A valid `customer_id` — mandates are always associated with a customer record
- Customer consent collected at your UI layer (checkbox, explicit agreement)
- For MIT compliance: `customer_acceptance` must include IP address and timestamp

---

## Phase 1: On-Session Setup (First Transaction)

### Save a Card with a Real Charge

```json
POST /payments
{
  "amount": 999,
  "currency": "USD",
  "confirm": true,
  "capture_method": "automatic",
  "customer_id": "cus_abc123",
  "setup_future_usage": "off_session",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  },
  "customer_acceptance": {
    "acceptance_type": "online",
    "accepted_at": "2024-06-15T10:00:00.000Z",
    "online": {
      "ip_address": "203.0.113.42",
      "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)..."
    }
  },
  "return_url": "https://yourapp.com/subscription/confirm"
}
```

**Response includes:**
```json
{
  "payment_id": "pay_abc123",
  "status": "succeeded",
  "mandate_id": "man_def456",
  "payment_method_id": "pm_ghi789"
}
```

Store `mandate_id` (or `payment_method_id`) in your database against the customer's subscription record.

---

### Zero-Amount Setup (No Charge at Enrollment)

Use when onboarding a subscription but not charging until the billing date:

```json
POST /payments
{
  "amount": 0,
  "currency": "USD",
  "confirm": true,
  "customer_id": "cus_abc123",
  "setup_future_usage": "off_session",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "03",
      "card_exp_year": "2030",
      "card_cvc": "737"
    }
  },
  "customer_acceptance": {
    "acceptance_type": "online",
    "accepted_at": "2024-06-15T10:00:00.000Z",
    "online": {
      "ip_address": "203.0.113.42",
      "user_agent": "Mozilla/5.0..."
    }
  },
  "mandate_data": {
    "customer_acceptance": {
      "acceptance_type": "online",
      "accepted_at": "2024-06-15T10:00:00.000Z",
      "online": {
        "ip_address": "203.0.113.42",
        "user_agent": "Mozilla/5.0..."
      }
    },
    "mandate_type": {
      "multi_use": {
        "amount": 100000,
        "currency": "USD",
        "start_date": "2024-06-15T00:00:00.000Z",
        "end_date": "2025-06-15T00:00:00.000Z"
      }
    }
  }
}
```

---

## Phase 2: Off-Session Charge (Recurring)

### Charge Using mandate_id

```json
POST /payments
{
  "amount": 999,
  "currency": "USD",
  "confirm": true,
  "capture_method": "automatic",
  "customer_id": "cus_abc123",
  "off_session": true,
  "mandate_id": "man_def456"
}
```

No `payment_method_data` needed — Hyperswitch uses the stored card from the mandate.

---

### Charge Using payment_token

If you stored a `payment_method_id` instead of a `mandate_id`:

```json
POST /payments
{
  "amount": 999,
  "currency": "USD",
  "confirm": true,
  "customer_id": "cus_abc123",
  "off_session": true,
  "payment_token": "pm_ghi789"
}
```

---

### List Customer's Saved Payment Methods

```bash
GET /customers/{customer_id}/payment_methods
```

Returns all saved cards/methods for the customer. Use this to let customers manage their payment methods.

---

## Mandate Types

| Type | Behavior |
|------|----------|
| `single_use` | One subsequent charge authorized — mandate expires after first use |
| `multi_use` | Multiple charges allowed, optionally bounded by `amount` cap and date range |

---

## setup_future_usage Values

| Value | Meaning | CIT/MIT Classification |
|-------|---------|----------------------|
| `on_session` | Save card; next charge will have customer present | CIT (customer-initiated) |
| `off_session` | Save card; subsequent charges are merchant-initiated | Sets up for MIT |

---

## Handling Soft Declines on Recurring Charges

Off-session charges can fail with soft declines. The correct recovery flow:

```
off-session charge fails (status: "failed", error_code: "do_not_honor")
         ↓
1. Notify customer by email/push: "Your payment failed"
2. Create a new on-session payment (bring customer back to app)
3. Customer re-enters card or confirms existing card
4. New mandate_id created — use this for subsequent off-session charges
```

Never retry a soft-declined MIT charge without bringing the customer back on-session — this escalates the decline rate.

---

## customer_acceptance Fields

| Field | Required | Notes |
|-------|----------|-------|
| `acceptance_type` | Yes | `online` or `offline` |
| `accepted_at` | Yes | ISO 8601 timestamp of when customer consented |
| `online.ip_address` | For `online` | Customer's IP at consent time |
| `online.user_agent` | For `online` | Browser User-Agent at consent time |

This data is required for SCA compliance and chargeback defense on recurring transactions.

---

## Production Tips

- Always capture `customer_acceptance` with real IP and timestamp — this is your legal proof of consent for recurring charges, critical for dispute defense.
- Store both `mandate_id` and `payment_method_id` — if mandate lookup fails, the payment_method may still be chargeable via `payment_token`.
- `off_session: true` is not just a hint — it tells the connector to use MIT exemption flags, which can affect authorization rates and SCA handling.
- Mandate IDs are connector-specific. If you switch a customer to a different connector, the mandate is not portable — you must re-enroll the customer.
- Implement a dunning strategy for failed recurring charges: retry after 3 days, then 7 days, then notify the customer before cancelling.
- Subscription billing dates and retry logic are your responsibility — Hyperswitch provides the payment rail, not the subscription scheduler.
