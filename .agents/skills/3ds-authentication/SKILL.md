---
name: 3ds-authentication
description: Use this skill when the user asks about "3D Secure", "3DS", "3DS2", "strong customer authentication", "SCA", "PSD2 compliance", "challenge flow", "frictionless flow", "external 3DS", "authentication_type", "complete_authorize", or needs to understand POST /payments/{id}/3ds/authentication and POST /payments/{id}/complete_authorize.
version: 1.0.0
---

# 3D Secure (3DS) Authentication

## When to Use

- Implementing SCA/PSD2 compliance for European payments
- Using an external 3DS provider (e.g., Netcetera, GPay 3DS) instead of the connector's built-in 3DS
- Handling the challenge flow where the cardholder must verify identity
- Completing authorization after external 3DS authentication

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/payments` | Create payment with `authentication_type: "three_ds"` |
| POST | `/payments/{payment_id}/3ds/authentication` | Submit external 3DS authentication data |
| POST | `/payments/{payment_id}/complete_authorize` | Complete authorization after redirect/3DS |

## 3DS Flow Types

### Frictionless Flow
Cardholder is not challenged — ACS authenticates in the background.
```
Create Payment → 3DS Check → Frictionless Auth → Capture
```

### Challenge Flow
Cardholder must complete an OTP or biometric challenge.
```
Create Payment → 3DS Check → Redirect to ACS → Challenge Complete → complete_authorize → Capture
```

## Common Scenarios

### 1. Create a 3DS-Enabled Payment

```json
POST /payments
{
  "amount": 5000,
  "currency": "EUR",
  "confirm": true,
  "authentication_type": "three_ds",
  "return_url": "https://yourapp.com/payment/3ds/complete",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "card_number": "4000000000003220",
      "card_exp_month": "12",
      "card_exp_year": "2025",
      "card_cvc": "737"
    }
  }
}
```
If a challenge is required, `next_action.type` = `"redirect_to_url"`. Redirect the user.

### 2. External 3DS — Submit Authentication Data

```json
POST /payments/{payment_id}/3ds/authentication
{
  "client_details": {
    "ip_address": "192.168.1.1",
    "user_agent": "Mozilla/5.0..."
  },
  "sdk_details": {
    "sdk_app_id": "app_123",
    "sdk_reference_number": "ref_456",
    "sdk_transaction_id": "txn_789"
  }
}
```

### 3. Complete Authorization After 3DS Redirect

```json
POST /payments/{payment_id}/complete_authorize
{
  "payment_id": "pay_abc123"
}
```
Called after the user returns to `return_url` following the 3DS challenge.

### 4. Handle Return URL

On your `return_url` page:
1. Extract `payment_id` from query params
2. Call `GET /payments/{payment_id}` to check `status`
3. If `status = "requires_customer_action"` — challenge not yet complete
4. If `status = "requires_capture"` or `"succeeded"` — authentication passed

## authentication_type Values

| Value | Meaning |
|-------|---------|
| `"three_ds"` | Force 3DS authentication |
| `"no_three_ds"` | Skip 3DS (use only where allowed) |

## Tips & Gotchas

- `return_url` is **required** when `authentication_type: "three_ds"` — missing it causes a 422.
- Test card `4000000000003220` always triggers a 3DS challenge in sandbox.
- Test card `4000000000003063` triggers frictionless 3DS (no redirect needed).
- After challenge completion, the browser redirects to `return_url?payment_id=pay_xxx`. Always verify on your server — don't trust client-side status.
- External 3DS (`POST /payments/{id}/3ds/authentication`) is only needed when you run your own 3DS server. For standard flows, Hyperswitch handles 3DS via the connector.
- Some connectors handle the `complete_authorize` step automatically via webhook — check connector-specific docs.
