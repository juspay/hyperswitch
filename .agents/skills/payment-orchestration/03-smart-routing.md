---
name: hyperswitch-smart-routing
description: Use this skill when the user asks about "smart routing", "connector routing", "which connector to use", "routing rules", "priority routing", "volume split", "least cost routing", "fallback connector", "retry on failure", "A/B test connectors", "route payments to Stripe vs Adyen", "routing field in payments request", "intelligent routing", or needs to configure how Hyperswitch selects payment processors.
version: 1.0.0
tags: [hyperswitch, routing, connectors, smart-routing, orchestration]
---

# Smart Routing

## Overview

Hyperswitch's routing engine selects which payment connector processes each transaction. You can use rule-based routing (priority, volume split, advanced rules), Hyperswitch's ML-powered success-rate routing, or override per-payment. This skill covers all routing modes and how to configure them.

## Prerequisites

- At least two connectors configured and enabled in the dashboard
- Each connector has a `merchant_connector_id` (MCA ID) — find it in **Connectors** in the dashboard

---

## Routing Hierarchy

When a payment is created, Hyperswitch selects a connector in this order:

```
1. Per-payment routing override  (routing field in request)
        ↓ (if absent)
2. Active routing algorithm       (set in Dashboard → Routing)
        ↓ (if absent)
3. Smart routing (ML)             (if enabled)
        ↓ (if absent)
4. Default connector              (fallback)
```

---

## Routing Types

### 1. Single Connector (Override)

Force a specific connector for one payment:

```json
POST /payments
{
  "amount": 5000,
  "currency": "USD",
  "confirm": true,
  "routing": {
    "type": "single",
    "data": {
      "connector": "stripe",
      "merchant_connector_id": "mca_01HXYZ"
    }
  },
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```

Use for: testing, compliance routing (certain currencies must go to specific processors), debugging.

---

### 2. Priority Routing

Try connectors in order; fall through to the next on failure:

```json
"routing": {
  "type": "priority",
  "data": [
    { "connector": "stripe",   "merchant_connector_id": "mca_01HXYZ" },
    { "connector": "adyen",    "merchant_connector_id": "mca_02HABC" },
    { "connector": "checkout", "merchant_connector_id": "mca_03HDEF" }
  ]
}
```

Hyperswitch attempts Stripe first. On hard decline or timeout, it retries with Adyen, then Checkout.

**When to use:** Maximize authorization rates with a primary connector and a reliable fallback.

---

### 3. Volume Split Routing

Distribute traffic by percentage — A/B testing, load distribution:

```json
"routing": {
  "type": "volume_split",
  "data": [
    { "connector": "stripe", "merchant_connector_id": "mca_01HXYZ", "split": 70 },
    { "connector": "adyen",  "merchant_connector_id": "mca_02HABC", "split": 30 }
  ]
}
```

Percentages must sum to exactly 100.

**When to use:** Comparing authorization rates between connectors, gradual connector migrations, load balancing.

---

### 4. Advanced Rule-Based Routing

Route based on payment attributes — amount, currency, card BIN, country:

Configure advanced rules in **Dashboard → Routing → Advanced** (YAML/JSON rule editor).

Example rule logic:
```
IF currency = "EUR" AND amount > 10000
  → adyen (mca_02HABC)
ELSE IF card.country = "IN"
  → razorpay (mca_04HGHI)
ELSE
  → stripe (mca_01HXYZ)
```

**When to use:** Cost optimization (route high-value EU transactions to Adyen for better rates), regulatory compliance, currency-specific processors.

---

## Dashboard Routing Configuration

To set a default routing algorithm for all payments (without per-payment overrides):

1. Open **Routing** in the Hyperswitch dashboard
2. Select **Create new routing** → choose type (Priority, Volume Split, or Advanced)
3. Configure connectors and rules
4. Click **Activate** — this becomes the active algorithm

Only one routing algorithm is active at a time. Previous algorithms are archived.

---

## Connector Fallback Behavior

Hyperswitch distinguishes between soft and hard failures:

| Failure Type | Behavior |
|-------------|----------|
| Network timeout | Retries with next connector in priority list |
| Hard decline (`card_declined`) | Does **not** automatically retry — stops at that connector |
| Connector downtime (5xx) | Retries with next connector |
| Authentication failure (wrong credentials) | Does not retry — configuration error |

> To enable automatic retry on soft declines, configure **Connectors → Retry** in the dashboard.

---

## Connector Categories

| Use Case | Recommended Connectors |
|----------|------------------------|
| Global (multi-currency) | `stripe`, `adyen`, `checkout`, `braintree`, `cybersource` |
| US-focused | `authorizedotnet`, `square`, `fiserv`, `nmi`, `stripe` |
| Europe | `adyen`, `mollie`, `multisafepay`, `redsys` (ES), `datatrans` (CH) |
| India | `razorpay`, `payu`, `cashfree`, `paytm` |
| APAC | `adyen`, `checkout`, `xendit` (SEA) |
| BNPL | `klarna`, `affirm`, `katapult` |
| Bank/ACH | `gocardless`, `dwolla`, `plaid` |

---

## Checking Which Connector Was Used

Every payment response includes `connector`:

```json
{
  "payment_id": "pay_abc123",
  "connector": "stripe",
  "connector_transaction_id": "ch_stripe_001",
  ...
}
```

---

## Production Tips

- `merchant_connector_id` is not the connector name — it's the specific MCA record in your account. One merchant can have multiple Stripe MCAs (e.g., US entity and EU entity).
- Volume split percentages are probabilistic — over millions of transactions the split converges; for small volumes it's approximate.
- Smart routing (ML) needs ~500+ transactions to produce reliable predictions. Enable it after onboarding, not on day one.
- For priority routing: put your lowest-cost or highest-success-rate connector first; most reliable connector last as the final safety net.
- Test fallback behavior in sandbox: intentionally use an invalid API key for your primary connector to verify the fallback fires.
- Routing overrides at the payment level bypass all dashboard configuration — use sparingly in production to avoid misconfiguration at scale.
