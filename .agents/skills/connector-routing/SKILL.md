---
name: connector-routing
description: Use this skill when the user asks about "connector routing", "which payment processor", "smart routing", "fallback connector", "retry on failure", "routing rules", "connector priority", "preferred connector", "configure routing", "route payments to Stripe vs Adyen", "least cost routing", or needs to understand how Hyperswitch selects payment connectors.
version: 1.0.0
---

# Connector Routing

## When to Use

- Choosing which payment processor handles a payment
- Setting up fallback/retry logic across multiple connectors
- Implementing cost-based or performance-based routing
- Overriding smart routing for a specific payment
- Understanding which of the 143+ connectors to use for a given scenario

## How Routing Works

Hyperswitch selects a connector through a priority chain:

```
1. Payment-level override  (routing field in request)
      ↓
2. Profile routing rules   (configured in dashboard)
      ↓
3. Smart routing           (ML-based success rate optimization)
      ↓
4. Default connector       (fallback)
```

## Specifying a Connector on a Payment

```json
POST /payments
{
  "amount": 1000,
  "currency": "USD",
  "confirm": true,
  "routing": {
    "type": "single",
    "data": {
      "connector": "stripe",
      "merchant_connector_id": "mca_abc123"
    }
  },
  "payment_method": "card",
  "payment_method_data": { "card": { ... } }
}
```

## Routing Types

| Type | Description | Use Case |
|------|-------------|----------|
| `single` | Route to one specific connector | Testing, compliance requirements |
| `priority` | Ordered list; fall to next on failure | Maximize reliability |
| `volume_split` | Split traffic by percentage | A/B testing, load distribution |
| `advanced` | Rule-based (amount, currency, country) | Cost optimization |

## Priority Routing (Fallback)

```json
"routing": {
  "type": "priority",
  "data": [
    { "connector": "stripe", "merchant_connector_id": "mca_001" },
    { "connector": "adyen", "merchant_connector_id": "mca_002" },
    { "connector": "checkout", "merchant_connector_id": "mca_003" }
  ]
}
```
Tries Stripe first; if it fails, falls back to Adyen, then Checkout.

## Volume Split Routing

```json
"routing": {
  "type": "volume_split",
  "data": [
    { "connector": "stripe", "split": 70 },
    { "connector": "adyen", "split": 30 }
  ]
}
```
Routes 70% of traffic to Stripe, 30% to Adyen.

## Key Connectors by Category

### Global / Multi-currency
`stripe`, `adyen`, `checkout`, `braintree`, `worldpay`, `cybersource`, `nuvei`

### APAC / India
`razorpay`, `paytm`, `phonepe`, `payu`, `cashfree`, `ccavenue`

### US-focused
`authorizedotnet`, `square`, `fiserv`, `nmi`, `stax`, `helcim`

### Europe
`mollie`, `multisafepay`, `klarna`, `trustpay`, `redsys` (Spain), `datatrans` (CH)

### BNPL (Buy Now Pay Later)
`klarna`, `affirm`, `afterpay` (via supported connectors), `katapult`

### Wallets
`paypal`, `amazonpay`, `applepay` / `googlepay` (via card connectors that support them)

### Crypto
`coinbase`, `coingate`, `opennode`, `cryptopay`

### Bank / ACH
`gocardless`, `plaid`, `dwolla`, `truelayer`

## Tips & Gotchas

- `merchant_connector_id` (`mca_xxx`) identifies a specific connector configuration in your Hyperswitch dashboard — one merchant can have multiple Stripe accounts (for different regions, etc.).
- Smart routing learns from historical data — give it at least a few hundred transactions before trusting its decisions.
- Not all connectors support all currencies and payment methods. Cross-reference [Hyperswitch connector matrix](https://hyperswitch.io/docs/connectors) before configuring routing rules.
- Volume split percentages must sum to 100.
- For priority routing, put your lowest-cost connector first and most reliable connector last as the final fallback.
- Routing overrides at the payment level bypass dashboard rules entirely — useful for testing but risky in production if misconfigured.
- Use `GET /account/payment_methods` to see which payment methods are enabled for your configured connectors.
