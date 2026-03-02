# Hanzo Payments

Open-source payment orchestration switch. Route payments across 50+ processors with smart retries, fallback, and unified analytics.

## Architecture

```
hanzo/commerce    Storefront, catalog, orders
       |
hanzo/payments    Payment routing (50+ processors)   <-- you are here
       |
hanzo/treasury    Ledger, reconciliation, wallets
       |
lux/treasury      On-chain treasury, MPC/KMS wallets
```

## Supported Processors

| Category | Processors |
|----------|-----------|
| **Cards** | Stripe, Adyen, Braintree, Checkout.com, Cybersource, Worldpay, NMI, Authorise.net, Square |
| **Bank** | Plaid, GoCardless, ACH (Column, Modern Treasury), SEPA, BACS |
| **Wallets** | Apple Pay, Google Pay, PayPal, Venmo, Cash App |
| **BNPL** | Klarna, Affirm, Afterpay, Sezzle |
| **Crypto** | Coinbase Commerce, BitPay, NOWPayments |
| **Regional** | Mercado Pago, Razorpay, Paytm, Mollie, iDEAL, Bancontact |
| **Wire** | Wise, CurrencyCloud, SWIFT, Fedwire |

## Features

- **Smart Routing** — Route to the optimal processor based on cost, success rate, and latency
- **Automatic Retries** — Cascade to backup processors on failure
- **Unified API** — Single integration for all payment methods
- **PCI DSS** — Vault-based card tokenization
- **3DS Authentication** — Native 3D Secure support
- **Multi-currency** — 135+ currencies with automatic FX
- **Webhooks** — Normalized events across all processors
- **Analytics** — Real-time payment analytics and reporting

## Quick Start

```bash
# Start with Docker
docker compose up -d

# Create a payment
curl -X POST http://localhost:8080/payments/create \
  -H "Content-Type: application/json" \
  -H "api-key: dev_key" \
  -d '{
    "amount": 1000,
    "currency": "USD",
    "payment_method": "card",
    "payment_method_data": {
      "card": {
        "card_number": "4242424242424242",
        "card_exp_month": "12",
        "card_exp_year": "2027",
        "card_cvc": "123"
      }
    },
    "connector": "stripe"
  }'
```

## Decision Engine

Smart routing rules defined in TOML:

```toml
[[rules]]
name = "route_high_value"
condition = "amount > 10000 AND currency == 'USD'"
action = "route"
connector = "adyen"
fallback = ["stripe", "checkout"]

[[rules]]
name = "route_eu"
condition = "country IN ['DE', 'FR', 'NL', 'BE']"
action = "route"
connector = "mollie"
fallback = ["adyen"]
```

## Integration with Hanzo Stack

Payments connects to:
- **hanzo/commerce** for order checkout and settlement
- **hanzo/treasury** for ledger recording and reconciliation
- **hanzo/vault** for PCI-compliant card tokenization
- **hanzo/kms** for API key and secret management

## Development

```bash
# Rust build
cargo build --release

# Run tests
cargo test

# Run with hot reload
cargo watch -x run
```

## License

Apache 2.0 — see [LICENSE](LICENSE)

