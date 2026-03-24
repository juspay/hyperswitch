---
name: recurring-mandates
description: Use this skill when the user asks about "recurring payments", "mandates", "subscriptions", "save card for later", "setup_future_usage", "off-session payments", "customer_acceptance", "MIT transactions", "merchant-initiated transactions", "charging a saved card", or needs to implement repeat billing without re-entering card details.
version: 1.0.0
---

# Recurring Payments and Mandates

## When to Use

- Saving a payment method for future use (subscriptions, stored credentials)
- Charging a customer off-session (without their active presence)
- Setting up a mandate to authorize recurring charges
- MIT (Merchant-Initiated Transaction) flows for billing

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/payments` | Create payment with `setup_future_usage` to save the method |
| POST | `/payments` | Subsequent charge using `mandate_id` or `payment_token` |
| GET | `/customers/{customer_id}/payment_methods` | List saved payment methods |

## Setup Flow

```
Step 1: On-session setup payment  →  save the card  →  get mandate_id / payment_token
Step 2: Off-session charge        →  use mandate_id or payment_token  →  no card input needed
```

## Essential Fields

| Field | Type | Notes |
|-------|------|-------|
| `setup_future_usage` | enum | `"on_session"` or `"off_session"` |
| `customer_id` | string | Required to associate the saved method |
| `mandate_data` | object | Define mandate terms for recurring billing |
| `off_session` | boolean | `true` for merchant-initiated charges |
| `mandate_id` | string | Use on subsequent charges (from step 1 response) |
| `payment_token` | string | Alternative to `mandate_id` for saved methods |
| `customer_acceptance` | object | For MIT, confirms customer consented |

## Common Scenarios

### 1. First Payment — Save Card for Future Use

```json
POST /payments
{
  "amount": 1000,
  "currency": "USD",
  "customer_id": "cus_abc123",
  "confirm": true,
  "setup_future_usage": "off_session",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "12",
      "card_exp_year": "2025",
      "card_cvc": "123"
    }
  },
  "customer_acceptance": {
    "acceptance_type": "online",
    "accepted_at": "2024-06-15T10:00:00Z",
    "online": {
      "ip_address": "192.168.1.1",
      "user_agent": "Mozilla/5.0..."
    }
  }
}
```
→ Response includes `mandate_id`. Save it for future charges.

### 2. Subsequent Off-Session Charge

```json
POST /payments
{
  "amount": 1000,
  "currency": "USD",
  "customer_id": "cus_abc123",
  "confirm": true,
  "off_session": true,
  "mandate_id": "man_abc123"
}
```
No card details needed — uses the saved mandate.

### 3. Setup Mandate Only (Zero-Dollar Auth)

```json
POST /payments
{
  "amount": 0,
  "currency": "USD",
  "customer_id": "cus_abc123",
  "confirm": true,
  "setup_future_usage": "off_session",
  "mandate_data": {
    "customer_acceptance": {
      "acceptance_type": "online",
      "accepted_at": "2024-06-15T10:00:00Z",
      "online": { "ip_address": "...", "user_agent": "..." }
    },
    "mandate_type": {
      "multi_use": {
        "amount": 10000,
        "currency": "USD"
      }
    }
  },
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```

### 4. Use Saved Payment Token

```json
// List customer's saved methods
GET /customers/{customer_id}/payment_methods

// Charge using a saved token
POST /payments
{
  "amount": 2000,
  "currency": "USD",
  "customer_id": "cus_abc123",
  "payment_token": "tok_abc123",
  "off_session": true,
  "confirm": true
}
```

## mandate_type Options

| Type | Use Case |
|------|----------|
| `single_use` | One-time authorization for a specific amount |
| `multi_use` | Recurring, multiple charges (subscriptions) |

## Tips & Gotchas

- Always capture `customer_acceptance` data (IP + timestamp) for SCA compliance and chargeback disputes.
- `setup_future_usage: "off_session"` tells the card network this will be used for future charges — triggers appropriate authorization flags.
- Off-session charges can fail due to soft declines (insufficient funds, expired card). Handle `status: "failed"` by re-engaging the customer on-session.
- Not all connectors support zero-dollar mandates — test in sandbox first.
- `mandate_id` is connector-specific — it cannot be transferred to a different connector.
- For subscriptions, store both `mandate_id` and a fallback `payment_token` in case mandate lookup fails.
