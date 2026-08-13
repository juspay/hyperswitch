---
name: hyperswitch-quickstart
description: Use this skill when the user asks "how do I get started with Hyperswitch", "first payment with Hyperswitch", "Hyperswitch sandbox setup", "get my first API key", "how do I test Hyperswitch", "integrate Hyperswitch from scratch", "quickstart guide", or needs a working end-to-end example from zero. Covers account creation, sandbox API keys, making a first payment, and verifying in the dashboard.
version: 1.0.0
tags: [hyperswitch, quickstart, onboarding, payments]
---

# Hyperswitch Quickstart

## Overview

Get from zero to a working payment in under 15 minutes. This guide covers sandbox account setup, API key retrieval, creating your first payment, and verifying the result — everything you need to validate Hyperswitch fits your stack.

## Prerequisites

- A Hyperswitch sandbox account ([app.hyperswitch.io](https://app.hyperswitch.io))
- `curl` or any HTTP client (Postman, Insomnia, your language SDK)
- Basic familiarity with REST APIs

---

## Step 1: Get Your API Key

1. Log in to [app.hyperswitch.io](https://app.hyperswitch.io)
2. Navigate to **Developers → API Keys**
3. Click **Create New API Key** → copy the value (shown once)

You will use this key in every request as:
```
api-key: YOUR_API_KEY
```

> **Sandbox vs Production**: Sandbox keys are prefixed differently and route to `sandbox.hyperswitch.io`. Never use production keys in test code.

---

## Step 2: Create Your First Payment

```bash
curl --request POST \
  --url https://sandbox.hyperswitch.io/payments \
  --header 'Content-Type: application/json' \
  --header 'api-key: YOUR_API_KEY' \
  --data '{
    "amount": 1000,
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
    "description": "My first Hyperswitch payment",
    "return_url": "https://example.com/payment/complete"
  }'
```

### Expected Response

```json
{
  "payment_id": "pay_xxxxxxxxxxxxxxxxxxxxxx",
  "status": "succeeded",
  "amount": 1000,
  "currency": "USD",
  "capture_method": "automatic",
  "payment_method": "card",
  "connector": "stripe",
  "created": "2024-06-15T10:30:00.000Z"
}
```

`status: "succeeded"` means the payment was authorized and captured in one step. ✓

---

## Step 3: Retrieve the Payment

```bash
curl --request GET \
  --url https://sandbox.hyperswitch.io/payments/pay_xxxxxxxxxxxxxxxxxxxxxx \
  --header 'api-key: YOUR_API_KEY'
```

---

## Step 4: Test Card Reference

| Card Number | Scenario |
|-------------|----------|
| `4242424242424242` | Successful payment |
| `4000000000000002` | Card declined |
| `4000000000003220` | 3DS challenge required |
| `4000000000003063` | 3DS frictionless (no redirect) |
| `4000000000009995` | Insufficient funds |
| `4000000000000069` | Expired card |

All test cards use any future expiry date and any 3-digit CVC.

---

## Step 5: Set Up Webhooks (Optional but Recommended)

Webhooks notify your server of async payment events. For local testing, use [smee.io](https://smee.io) or [ngrok](https://ngrok.com):

```bash
# Using smee.io
npx smee -u https://smee.io/your-channel -t http://localhost:3000/webhooks
```

Register the webhook URL in **Developers → Webhooks** in the dashboard.

Key events to handle:
- `payment.succeeded` — trigger fulfillment
- `payment.failed` — notify the customer
- `refund.succeeded` — update order state

---

## Step 6: Verify in Dashboard

1. Open [app.hyperswitch.io](https://app.hyperswitch.io)
2. Navigate to **Payments**
3. Find your `payment_id` — status should show **Succeeded**

---

## Common First-Run Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `401 Unauthorized` | Invalid or missing `api-key` header | Confirm the key is copied fully, no trailing space |
| `422 Unprocessable Entity` | Missing required fields | Check that `amount`, `currency`, `payment_method`, and `payment_method_data` are all present |
| `connector_error` | No connector configured | Add and enable at least one connector in **Connectors** in the dashboard |
| `amount must be positive` | `amount: 0` or negative | `amount` is in smallest currency unit — $10 = `1000` |

---

## What's Next

- [Create Payment](./01-create-payment.md) — auth-only, 3DS, wallets, metadata
- [Smart Routing](./03-smart-routing.md) — route across multiple connectors
- [Webhooks](./04-webhook-handling.md) — reliable event processing
