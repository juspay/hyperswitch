---
name: create-payment
description: Use this skill when the user wants to "create a payment", "initiate a payment", "charge a customer", "start a transaction", "make a payment request", "build a PaymentsRequest", or needs to understand the POST /payments endpoint. Covers one-time charges, auth-only flows, auth+capture, 3DS-enabled payments, metadata, customer fields, and connector selection.
version: 1.0.0
---

# Create a Payment

## When to Use

- Creating a new payment (one-time charge, subscription first charge, marketplace split)
- Choosing between `capture_method: automatic` vs `manual`
- Attaching customer data, metadata, or billing address
- Specifying a preferred connector or letting smart routing decide

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/payments` | Create a new payment |
| POST | `/payments/{payment_id}/confirm` | Confirm a payment created in `requires_confirmation` state |
| POST | `/payments/{payment_id}` | Update a pending payment |

## Essential Fields

| Field | Type | Notes |
|-------|------|-------|
| `amount` | integer | In smallest currency unit (cents for USD) |
| `currency` | string | ISO 4217, e.g. `"USD"` |
| `capture_method` | enum | `automatic` (charge immediately) or `manual` (auth only) |
| `confirm` | boolean | `true` to immediately confirm; `false` creates in `requires_confirmation` |
| `payment_method` | enum | `card`, `bank_redirect`, `wallet`, etc. |
| `payment_method_data` | object | Actual card/wallet details |
| `customer_id` | string | Links to an existing customer |
| `return_url` | string | Required for redirect-based payment methods |
| `metadata` | object | Arbitrary key-value pairs (max 255 chars per value) |
| `description` | string | Shows on payment receipts |
| `statement_descriptor_name` | string | Appears on bank statement |

## Common Scenarios

### 1. Immediate Charge (auth + capture)

```json
POST /payments
{
  "amount": 1000,
  "currency": "USD",
  "capture_method": "automatic",
  "confirm": true,
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4242424242424242",
      "card_exp_month": "12",
      "card_exp_year": "2025",
      "card_cvc": "123"
    }
  }
}
```

### 2. Auth Only (capture later)

```json
POST /payments
{
  "amount": 5000,
  "currency": "USD",
  "capture_method": "manual",
  "confirm": true,
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```
→ Returns `status: "requires_capture"`. Capture via `POST /payments/{id}/capture`.

### 3. Two-Step: Create then Confirm

```json
POST /payments
{
  "amount": 2000,
  "currency": "EUR",
  "confirm": false
}
// Returns payment_id with status: "requires_confirmation"

POST /payments/{payment_id}/confirm
{
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```

### 4. With Customer and Metadata

```json
POST /payments
{
  "amount": 3000,
  "currency": "GBP",
  "customer_id": "cus_abc123",
  "email": "user@example.com",
  "metadata": { "order_id": "ORD-999", "source": "mobile_app" },
  "description": "Order #ORD-999",
  "capture_method": "automatic",
  "confirm": true,
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```

### 5. Wallet / Redirect Payment

```json
POST /payments
{
  "amount": 1500,
  "currency": "USD",
  "return_url": "https://yourapp.com/payment/complete",
  "confirm": true,
  "payment_method": "wallet",
  "payment_method_data": {
    "wallet": { "paypal_redirect": {} }
  }
}
```
→ Response includes `next_action.redirect_to_url`. Redirect the user there.

## Tips & Gotchas

- `amount` is always in the **smallest currency unit** — $10.00 = `1000`
- `confirm: true` + `capture_method: automatic` = single-step charge. Most common for e-commerce.
- If `confirm: false`, the payment stays in `requires_confirmation` state until explicitly confirmed.
- `return_url` is **required** for wallets, bank redirects, and 3DS flows. Omitting it causes a 422.
- Sandbox test cards: `4242424242424242` (success), `4000000000000002` (decline).
- Idempotency: pass `Idempotency-Key` header to safely retry without double-charging.
- `metadata` is not visible to the customer — use `description` for statement-visible text.
