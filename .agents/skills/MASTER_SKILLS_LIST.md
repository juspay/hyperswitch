# Master Skills List

Complete roadmap for Hyperswitch AI coding skills. Skills marked ✅ are available now. Skills marked 📋 are planned.

## Current Skills (13)

| # | Skill | Category | Status |
|---|-------|----------|--------|
| 1 | `payment-orchestration/00-quickstart` | Core | ✅ Done |
| 2 | `payment-orchestration/01-create-payment` | Core | ✅ Done |
| 3 | `payment-orchestration/02-refunds` | Core | ✅ Done |
| 4 | `payment-orchestration/03-smart-routing` | Core | ✅ Done |
| 5 | `payment-orchestration/04-webhook-handling` | Core | ✅ Done |
| 6 | `payment-orchestration/05-mandates-recurring` | Core | ✅ Done |
| 7 | `payment-orchestration/06-payment-links` | Core | ✅ Done |
| 8 | `connectors/00-connector-setup` | Connectors | ✅ Done |
| 9 | `connectors/01-stripe-deep-dive` | Connectors | ✅ Done |
| 10 | `connectors/02-adyen-deep-dive` | Connectors | ✅ Done |
| 11 | `sdk/01-react-integration` | SDK | ✅ Done |
| 12 | `vault/00-vault-overview` | Security | ✅ Done |
| 13 | `demo-store/00-demo-store-overview` | Examples | ✅ Done |

---

## Planned Skills (50+)

### Payment Orchestration (continued)

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 14 | `payment-orchestration/07-incremental-auth` | High | Hotels, car rentals, hospitality |
| 15 | `payment-orchestration/08-3ds-external` | High | External 3DS server integration (Netcetera, GPay 3DS) |
| 16 | `payment-orchestration/09-payment-methods-management` | High | Create, list, update, delete payment methods |
| 17 | `payment-orchestration/10-multi-currency` | Medium | Dynamic currency conversion, settlement currency |
| 18 | `payment-orchestration/11-split-payments` | Medium | Marketplace split, platform fees |
| 19 | `payment-orchestration/12-payment-attempts` | Medium | Retry logic, multiple attempts per payment |
| 20 | `payment-orchestration/13-session-tokens` | Medium | SDK session token flow for wallets |

### Connectors (continued)

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 21 | `connectors/03-braintree-deep-dive` | High | PayPal ecosystem, Venmo |
| 22 | `connectors/04-checkout-deep-dive` | High | UK/EU focus |
| 23 | `connectors/05-razorpay-deep-dive` | High | India payments — UPI, wallets |
| 24 | `connectors/06-cybersource-deep-dive` | Medium | Enterprise US market |
| 25 | `connectors/07-paypal-direct` | Medium | PayPal as primary processor |
| 26 | `connectors/08-klarna-deep-dive` | Medium | BNPL configuration |
| 27 | `connectors/09-gocardless-deep-dive` | Medium | SEPA/BACS/ACH direct debit |
| 28 | `connectors/10-connector-testing-guide` | Medium | How to test any connector in sandbox |

### SDK (continued)

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 29 | `sdk/02-nextjs-integration` | High | Next.js App Router + Server Components |
| 30 | `sdk/03-vanilla-js-integration` | High | Plain HTML/JS, no framework |
| 31 | `sdk/04-ios-sdk` | Medium | Native iOS (Swift) |
| 32 | `sdk/05-android-sdk` | Medium | Native Android (Kotlin) |
| 33 | `sdk/06-flutter-integration` | Medium | Flutter cross-platform |
| 34 | `sdk/07-react-native-integration` | Medium | React Native |
| 35 | `sdk/08-web-components` | Low | Framework-agnostic web components |

### Vault & Security (continued)

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 36 | `vault/01-network-tokenization` | High | Visa VTS, Mastercard MDES |
| 37 | `vault/02-self-hosted-locker` | High | Deploy `hyperswitch-card-vault` |
| 38 | `vault/03-pci-compliance-guide` | Medium | SAQ types, scope reduction checklist |

### Analytics & Operations

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 39 | `analytics/00-payment-analytics` | High | Metrics, success rates, connector comparison |
| 40 | `analytics/01-reconciliation` | Medium | Settlement reconciliation, export APIs |
| 41 | `analytics/02-fraud-risk` | Medium | Risk signals, fraud prevention |

### Platform & Infrastructure

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 42 | `platform/00-self-hosting` | High | Docker, Kubernetes deployment |
| 43 | `platform/01-configuration` | High | `config/` deep dive, feature flags |
| 44 | `platform/02-database-migrations` | Medium | Running migrations, schema overview |
| 45 | `platform/03-monitoring` | Medium | Grafana dashboards, alerting |
| 46 | `platform/04-rate-limiting` | Low | API rate limits, retry-after |

### Advanced Features

| # | Skill | Priority | Notes |
|---|-------|----------|-------|
| 47 | `advanced/00-disputes-deep-dive` | High | Full dispute lifecycle, evidence API |
| 48 | `advanced/01-unified-authentication` | High | External authentication service |
| 49 | `advanced/02-dynamic-tax` | Medium | Tax calculation integration |
| 50 | `advanced/03-revenue-recovery` | Medium | Failed payment recovery flows |
| 51 | `advanced/04-account-management` | Medium | Merchant accounts, profiles, API keys |
| 52 | `advanced/05-idempotency` | Medium | Idempotency patterns and best practices |
| 53 | `advanced/06-testing-strategies` | Medium | Integration testing, sandbox patterns |

---

## Contributing a New Skill

See [`SKILL_FORMAT.md`](./SKILL_FORMAT.md) for authoring guidelines.

Priority is determined by:
1. **Frequency of developer questions** in GitHub issues and Discord
2. **Integration complexity** — harder flows need better guidance
3. **Business impact** — skills that improve authorization rates or reduce integration time
