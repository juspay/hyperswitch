---
name: hyperswitch-connector-setup
description: Use this skill when the user asks "how do I add a connector", "configure a payment processor", "what is a merchant connector account", "MCA ID", "connector credentials", "test mode vs live mode", "enable a connector", "add Stripe to Hyperswitch", "add Adyen to Hyperswitch", "which connectors does Hyperswitch support", or needs to onboard a new payment processor.
version: 1.0.0
tags: [hyperswitch, connectors, setup, onboarding, MCA]
---

# Connector Setup

## Overview

A connector in Hyperswitch is an integration with a payment processor (Stripe, Adyen, Checkout.com, etc.). Each connector configuration creates a **Merchant Connector Account (MCA)** — a record that stores credentials, enabled payment methods, and routing metadata. One merchant can have multiple MCAs for the same connector (e.g., separate Stripe accounts for US and EU).

## Prerequisites

- Active account with the target payment processor
- API credentials from that processor (typically an API key or key pair)
- Hyperswitch sandbox account with admin access

---

## Step 1: Add a Connector in the Dashboard

1. Log in to [app.hyperswitch.io](https://app.hyperswitch.io)
2. Navigate to **Connectors → Payment Processors**
3. Click **Connect** next to your target connector
4. Enter the connector's API credentials (see per-connector details below)
5. Select **Test Mode** for sandbox; **Live Mode** for production
6. Enable the payment methods you want to accept
7. Click **Proceed** — this creates an MCA and returns a `merchant_connector_id`

---

## Step 2: Understand the merchant_connector_id

Every connector configuration has a unique `merchant_connector_id` (format: `mca_01HXXX...`). This ID is used in:

- Per-payment routing overrides (`routing.data.merchant_connector_id`)
- Priority routing arrays
- Debugging (payment response includes `connector` + `merchant_connector_id`)

**Find your MCA ID:** Dashboard → Connectors → click the connector → copy the ID from the URL or details panel.

---

## Supported Connectors (143+)

### Tier 1 — Global Coverage

| Connector | Supports | Notes |
|-----------|---------|-------|
| `stripe` | Cards, wallets, BNPL, bank debits | Best sandbox tooling |
| `adyen` | Cards, local methods, wallets | Best for EU/APAC coverage |
| `checkout` | Cards, wallets, APMs | Strong UK/Europe presence |
| `braintree` | Cards, PayPal, Venmo | PayPal ecosystem |
| `cybersource` | Cards, APMs | Strong US enterprise |
| `worldpay` | Cards, wallets | Global enterprise |

### Regional

| Connector | Region | Specialization |
|-----------|--------|---------------|
| `razorpay` | India | UPI, wallets, net banking |
| `payu` | India, LATAM, CEE | Multi-region |
| `paystack` | Africa | Nigeria, Ghana, Kenya |
| `xendit` | SEA | Indonesia, Philippines |
| `redsys` | Spain | Local card schemes |
| `datatrans` | Switzerland | PostFinance, TWINT |

### BNPL

| Connector | Products |
|-----------|---------|
| `klarna` | Pay Now, Pay Later, Financing |
| `affirm` | Installments (US) |
| `katapult` | Lease-to-own |

### Bank / ACH

| Connector | Method |
|-----------|--------|
| `gocardless` | SEPA, BACS, ACH |
| `plaid` | ACH via bank linking |
| `dwolla` | ACH transfers |

---

## Credential Reference

### Stripe

```
API Key (Secret):  sk_test_...  (test) / sk_live_...  (live)
Webhook Secret:    whsec_...    (for Stripe → Hyperswitch webhook forwarding)
```

### Adyen

```
API Key:           AQE...
Merchant Account:  YourMerchantAccount
HMAC Key:          (for webhook signature verification)
```

### Checkout.com

```
Secret Key:        sk_test_...
Public Key:        pk_test_...
```

### Braintree

```
Merchant ID
Public Key
Private Key
Environment: sandbox / production
```

---

## Test Mode vs Live Mode

| Setting | Behavior |
|---------|----------|
| **Test Mode** | Uses connector's sandbox environment. Real credentials may work but no actual money moves. |
| **Live Mode** | Uses connector's production environment. Real charges apply. |

> Your Hyperswitch sandbox account should always use connectors in **Test Mode**. Only switch a connector to Live Mode when you are ready to process real payments on a production Hyperswitch account.

---

## Enabling Payment Methods

After adding credentials, explicitly enable the payment methods you accept:

1. Dashboard → Connectors → click your connector
2. **Payment Methods** tab
3. Toggle on: Cards, Wallets, Bank Redirects, etc.
4. For card networks: enable Visa, Mastercard, Amex as needed

Only enabled payment methods appear in the Hyperswitch checkout SDK and are accepted via the API.

---

## Multiple Connector Accounts

You can add the same connector multiple times (e.g., two Stripe accounts):

| MCA | Purpose |
|-----|---------|
| `mca_stripe_us` | US entity — USD transactions |
| `mca_stripe_eu` | EU entity — EUR transactions, handles SCA |

Use advanced routing rules to route based on currency or customer country.

---

## Production Tips

- Store connector credentials in a secrets manager (AWS Secrets Manager, Vault, GCP Secret Manager) — never in source code or environment files committed to git.
- Enable **only** the payment methods you actually support in your checkout UI — stray-enabled methods cause confusing declines.
- Test credential rotation in sandbox first: update credentials in dashboard → verify a test payment succeeds → then rotate in production.
- Set up connector-level webhooks where supported (e.g., Stripe webhooks to Hyperswitch) for real-time dispute and refund notifications from the connector.
- Each MCA has its own success rate metrics in the dashboard — review per-connector analytics monthly to identify underperforming connectors.
