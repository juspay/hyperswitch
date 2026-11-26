---
name: hyperswitch-vault
description: Use this skill when the user asks about "Hyperswitch vault", "card tokenization", "PCI compliance", "store a card securely", "payment_token", "network tokenization", "locker", "PCI DSS scope", "tokenize a card", "secure card storage", "retrieve saved cards", or needs to understand how Hyperswitch handles sensitive payment data.
version: 1.0.0
tags: [hyperswitch, vault, tokenization, PCI, security, locker]
---

# Vault & Tokenization

## Overview

The Hyperswitch Vault (also called the Locker) stores payment method data — primarily card PANs — as opaque tokens. When a card is saved via Hyperswitch, your application only ever sees and stores the `payment_method_id` (a token). The actual card number lives encrypted in the vault. This removes your application from PCI DSS scope for cardholder data.

## PCI Scope Impact

| Without Hyperswitch Vault | With Hyperswitch Vault |
|--------------------------|----------------------|
| Your server handles raw PANs | Your server handles only tokens |
| PCI DSS SAQ D or full audit | PCI DSS SAQ A (minimal scope) |
| You implement encryption, key rotation, HSMs | Hyperswitch handles all of this |

---

## How Tokenization Works

```
Customer enters card → PaymentElement (iframe)
         ↓
Hyperswitch SDK → Vault → stores encrypted PAN → returns payment_method_id
         ↓
Your server stores payment_method_id (a token, not PAN)
         ↓
Future charge: POST /payments { payment_token: "pm_abc123" }
         ↓
Hyperswitch Vault → retrieves PAN → sends to connector → returns result
```

Your server never sees the raw card number at any point.

---

## Save a Card (On-Session)

```json
POST /payments
{
  "amount": 1000,
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
  }
}
```

**Response includes:**
```json
{
  "payment_method_id": "pm_abc123",
  "payment_method": "card",
  "payment_method_data": {
    "card": {
      "last4": "4242",
      "exp_month": "03",
      "exp_year": "2030",
      "card_network": "Visa",
      "card_type": "credit"
    }
  }
}
```

Note: `payment_method_data.card` in the response contains only the last4, expiry, and network — never the full PAN.

---

## List Saved Payment Methods

```bash
GET /customers/{customer_id}/payment_methods
```

**Response:**
```json
{
  "customer_payment_methods": [
    {
      "payment_token": "pm_abc123",
      "payment_method": "card",
      "card": {
        "last4_digits": "4242",
        "exp_month": "03",
        "exp_year": "2030",
        "card_network": "Visa",
        "nick_name": "My Visa Card"
      },
      "recurring_enabled": true,
      "installment_payment_enabled": false,
      "created": "2024-06-15T10:00:00.000Z"
    }
  ]
}
```

---

## Charge a Saved Card

```json
POST /payments
{
  "amount": 4999,
  "currency": "USD",
  "confirm": true,
  "customer_id": "cus_abc123",
  "off_session": true,
  "payment_token": "pm_abc123"
}
```

No card details needed — Hyperswitch retrieves from vault.

---

## Delete a Saved Payment Method

```bash
DELETE /payment_methods/{payment_method_id}
```

This permanently removes the token from the vault. Subsequent charges using this `payment_token` will fail.

---

## Update a Payment Method

```bash
POST /payment_methods/{payment_method_id}/update
{
  "card": {
    "card_exp_month": "08",
    "card_exp_year": "2027"
  },
  "nick_name": "Work Visa"
}
```

---

## Network Tokenization

Network tokenization replaces the actual PAN with a network-issued token (DPAN) provided by Visa (VTS) or Mastercard (MDES). Benefits:

- Higher authorization rates (15–20% improvement on some connectors)
- Automatic card updates when a card is reissued
- Additional fraud signals shared with the network

Network tokenization is available for supported connectors. Enable it in **Dashboard → Settings → Network Tokenization**.

When enabled:
1. On first save, Hyperswitch provisions a DPAN from the card network
2. All subsequent charges use the DPAN instead of the PAN
3. When the issuer reissues the card (new expiry, new number), the network updates the DPAN automatically — no customer action needed

---

## Self-Hosted Vault (Locker)

Hyperswitch Vault is also available as an open-source, self-hosted component:

- GitHub: [juspay/hyperswitch-card-vault](https://github.com/juspay/hyperswitch-card-vault)
- Deploy in your own infrastructure for maximum data sovereignty
- Uses AES-256 encryption with a JWE-wrapped DEK
- Supports AWS KMS, GCP KMS, or local key management

Configure self-hosted locker in Hyperswitch:
```toml
# config/development.toml
[locker]
host = "https://your-locker-instance.internal"
mock_locker = false
locker_signing_key_id = "1"
```

---

## Security Properties

| Property | Detail |
|----------|--------|
| Encryption | AES-256-GCM |
| Key management | Per-tenant DEK, wrapped with master key (KMS) |
| Access control | Only your merchant account can retrieve your tokens |
| Audit logs | Every token read/write is logged |
| Data residency | Configurable (US, EU, or self-hosted) |
| PCI DSS certification | PCI DSS Level 1 compliant |

---

## Production Tips

- Always store `payment_method_id` in your database immediately after the first payment — you cannot retrieve it again from the Hyperswitch API; it is returned only once in the payment response.
- Implement a "manage payment methods" UI for your customers — let them delete old cards and add new ones. This reduces churn from expired cards.
- Set `nick_name` on saved cards (e.g., "Personal Visa ending 4242") to help customers identify their saved methods in your UI.
- For subscription billing, combine network tokenization with mandate management to maximize authorization rates on recurring charges.
- `payment_token` and `mandate_id` serve different purposes: `payment_token` is for re-presenting a card; `mandate_id` is for off-session MIT transactions. Use `mandate_id` when charging without customer presence for SCA/MIT compliance.
