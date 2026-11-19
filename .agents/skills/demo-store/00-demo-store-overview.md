---
name: hyperswitch-demo-store
description: Use this skill when the user asks about "Hyperswitch demo", "demo store", "example app", "sample integration", "Postman collection", "test Hyperswitch with Postman", "run the demo locally", "Next.js Hyperswitch example", "see Hyperswitch in action", "hyperswitch-demo-store repo", or wants to explore working examples before building their own integration.
version: 1.0.0
tags: [hyperswitch, demo, examples, postman, nextjs, quickstart]
---

# Demo Store & Example Apps

## Overview

Before writing your own integration, explore the official Hyperswitch demo apps and Postman collections. They show working end-to-end flows with real API calls, letting you understand the integration patterns before writing any code.

---

## 1. Hyperswitch Demo Store

A full-featured e-commerce demo built with Next.js showcasing the Hyperswitch React SDK.

**Repository:** [github.com/juspay/hyperswitch-demo-store](https://github.com/juspay/hyperswitch-demo-store)

### Run Locally

```bash
git clone https://github.com/juspay/hyperswitch-demo-store.git
cd hyperswitch-demo-store
npm install

# Create .env.local
cat > .env.local << EOF
NEXT_PUBLIC_HYPERSWITCH_PUBLISHABLE_KEY=pk_snd_...
HYPERSWITCH_SECRET_KEY=snd_...
HYPERSWITCH_SERVER_URL=https://sandbox.hyperswitch.io
EOF

npm run dev
```

Open [localhost:3000](http://localhost:3000) to browse the demo store.

### What It Demonstrates

- Product catalog and cart
- Checkout page using `HyperElements` + `PaymentElement`
- Server-side payment creation endpoint
- 3DS redirect handling
- Order confirmation after `payment.succeeded` webhook
- Saved card reuse for returning customers
- Multiple payment method tabs (card, wallet, bank redirect)

---

## 2. Postman Collection

The official Postman collection covers every Hyperswitch API endpoint with pre-configured examples.

**Location in repo:** [`/postman/`](https://github.com/juspay/hyperswitch/tree/main/postman)

### Import into Postman

1. Open Postman → **Import**
2. Select `postman/collection.json` from the repo
3. Set environment variables:

| Variable | Value |
|----------|-------|
| `baseUrl` | `https://sandbox.hyperswitch.io` |
| `admin_api_key` | Your admin API key |
| `api_key` | Your merchant API key |
| `merchant_id` | Your merchant ID (from dashboard) |

4. Run the **Environment Setup** folder first — it creates test customers, payment methods, and connectors
5. Execute flows sequentially:
   - **Payments** → Create → Confirm → Capture → Refund
   - **Customers** → Create → Add Payment Method → List
   - **Routing** → Configure → Test

### Automated Test Runner

```bash
# Run full Postman collection with Newman
npm install -g newman

newman run postman/collection.json \
  --environment postman/env.json \
  --reporters cli,json \
  --reporter-json-export postman/results.json
```

---

## 3. Cypress E2E Tests

The repo includes a full Cypress test suite against a running Hyperswitch instance:

**Location:** `/cypress-tests/` and `/cypress-tests-v2/`

```bash
cd cypress-tests
npm install

# Set credentials in cypress.env.json
npx cypress run --spec "cypress/e2e/payments/**"
```

The Cypress tests are an excellent reference for understanding the complete payment flow from a browser's perspective.

---

## 4. Load Testing Scripts

For performance benchmarking:

**Location:** `/loadtest/`

```bash
cd loadtest
# Uses k6 for load testing
k6 run scripts/payments.js --vus 50 --duration 60s
```

---

## 5. Integration Examples by Use Case

| Use Case | Reference |
|----------|-----------|
| Basic card payment (Node.js) | `postman/` folder — Payments collection |
| React SDK checkout | `hyperswitch-demo-store` repo |
| Webhook handler | `postman/` → Webhooks folder |
| Routing configuration | `postman/` → Routing folder |
| Connector setup | `cypress-tests/` → connector setup specs |

---

## Quick Sandbox Credentials

Get sandbox credentials quickly:

1. Visit [app.hyperswitch.io](https://app.hyperswitch.io) → sign up
2. Navigate to **Developers → API Keys** → create a key
3. Add a test connector: **Connectors → Stripe** → enter your Stripe test secret key (`sk_test_...` from [dashboard.stripe.com](https://dashboard.stripe.com) → Developers → API Keys)
4. Run `POST /payments` with `card_number: 4242424242424242` — you have a working integration

---

## Production Tips

- The demo store is intentionally minimal — it does not implement idempotency keys, webhook signature verification, or database persistence. Add these before going to production.
- The Postman collection uses `{{payment_id}}` variables that chain between requests — run requests in order, not individually.
- Cypress tests require a running Hyperswitch server (local or sandbox). Point `CYPRESS_BASE_URL` to your instance.
