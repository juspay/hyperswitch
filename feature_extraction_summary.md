# Hyperswitch Feature Extraction — Summary

Generated from `python3 scripts/extract_features.py` against the current codebase.

---

## At a Glance

| | Bucket 1 | Bucket 2 | Bucket 3 |
|---|---|---|---|
| **Name** | Connector + Flow | Connector + PM + PMT | Core Flow |
| **Total rows** | 98 | 858 | 99 |
| **Unique features** | 26 | 4 | 99 |
| **Unique connectors** | 34 | 88 | — |
| **Unique PM types** | — | 99 | — |
| **Cypress covered** | 8 (8%) | 312 (36%) | 28 (28%) |
| **Cypress not\_covered** | 76 (78%) | 414 (48%) | 71 (72%) |
| **No cypress config** | 14 (14%) | 132 (15%) | — |

---

## Bucket 1 — Connector × Feature (98 rows)

Payload-level features that differ per connector, PM-agnostic. Detection via transformer field usage and connector-specific flow macro exclusions.

### Feature coverage

| Feature | Connectors | Cypress |
|---|---|---|
| Preprocessing Flow | 16 | not\_covered |
| Network Transaction ID | 15 | not\_covered |
| Pre-Authentication Flow | 8 | not\_covered |
| Billing Descriptor | 7 | not\_covered |
| Incremental Authorization | 5 | 5 covered |
| Order Create Flow | 5 | not\_covered |
| Post-Authentication Flow | 4 | not\_covered |
| Authentication Flow | 3 | not\_covered |
| Dispute Accept | 3 | not\_covered |
| L2/L3 Data Processing | 3 | not\_covered |
| Dispute Defend | 2 | not\_covered |
| Extended Authorization | 2 | not\_covered |
| Partner Merchant Identifier | 2 | not\_covered |
| Partial Authorization | 3 | not\_covered |
| Split Payments | 3 | not\_covered |
| Split Refunds | 3 | not\_covered |
| Step Up Authentication | 2 | not\_covered |
| Connector Intent Metadata | 1 | no\_cypress\_config |
| Connector Testing Data | 1 | not\_covered |
| Overcapture | 1+ | covered |
| QR Code Generation Flow | 1 | no\_cypress\_config |
| Push Notification Flow | 1 | no\_cypress\_config |
| Balance Check Flow | 1–2 | not\_covered |
| Settlement Split Call | 1 | not\_covered |
| Surcharge | 1–2 | not\_covered |
| Revenue Recovery | billing connectors | not\_covered |

### Cypress summary

- **4 features with any coverage**: Incremental Authorization, Overcapture, Network Transaction ID (NTID proxy), Installments
- **22 features with zero cypress coverage** — primary gap area for Bucket 1

---

## Bucket 2 — Connector × PM × PMT × Feature (858 rows)

Behavior differs per connector AND per payment method type. Sourced from `SUPPORTED_PAYMENT_METHODS` static + wallet decrypt variant detection.

### Features tracked per (connector, PM, PMT)

| Feature | Description |
|---|---|
| Payment | Base payment flow |
| Refund | Refund supported (from `FeatureStatus::Supported`) |
| Mandate | Mandate supported (from `FeatureStatus::Supported`) |
| Payment (Decrypt Flow) | Wallet pre-decrypted token path (ApplePay/GooglePay/Paze) |

### Payment method breakdown

| Payment Method | PMTs | Row count |
|---|---|---|
| Card | Credit, Debit, … | ~277 |
| Wallet | ApplePay, GooglePay, Paypal, Paze, … | ~300+ |
| BankRedirect | Ideal, Eps, Giropay, Sofort, … | ~120 |
| BankTransfer | SEPA, ACH, … | ~60 |
| PayLater | Klarna, Afterpay, … | ~40 |
| Others | BankDebit, Crypto, Voucher, GiftCard, etc. | ~60 |

### Top payment method types by row count

| PMT | Rows |
|---|---|
| Credit | 142 |
| Debit | 135 |
| ApplePay | 86 |
| GooglePay | 84 |
| Paypal | 28 |
| Ideal | 24 |
| Paze | 22 |
| Eps | 19 |
| Giropay | 19 |
| Sofort | 17 |

### Wallet decrypt flow variants (+60 rows)

| Wallet PMT | Connectors |
|---|---|
| ApplePay Decrypt | 23 connectors |
| GooglePay Decrypt | 20 connectors |
| Paze Decrypt | 17 connectors |

Key connectors: adyen, bankofamerica, barclaycard, braintree, checkout, cybersource, finix, fiserv, fiuu, mollie, nmi, nuvei, payme, paysafe, square, stax, stripe, wellsfargo, worldpayvantiv

### Cypress summary

- **52 connectors** have at least 1 covered row
- **30 connectors** have no cypress config at all
- Card (Credit/Debit) payments have the best coverage; wallet/bank-redirect have gaps
- Mandates and Refunds are mostly `not_covered` outside of card flows

---

## Bucket 3 — Core Features (99 rows)

Same behavior regardless of connector. Test once. Configured via merchant account, business profile, or admin configs API.

### Feature source distribution

| Source | Features | Notes |
|---|---|---|
| `business_profile` | 32 | `POST /business_profile` |
| `configs table` | 24 | `POST /configs` + `POST /configs/{key}` — admin key-value store |
| `api_models/payments` | 9 | Payment request/response fields |
| `api_models/payouts` | 5 | Payout-specific |
| `merchant_account` | 4 | `POST /accounts` |
| `superposition` | 3 | Runtime feature flags via Superposition service |
| `api_models/admin` | 3 | Admin-level configs |
| Others | 19 | Refunds, customers, routing, relay, etc. |

### Cypress summary

| Status | Count |
|---|---|
| covered | 28 (28%) |
| not\_covered | 71 (72%) |

### Well-tested core features (covered)
Customer Management, Routing Algorithm, Blocklist, PM Filters CGraph, Eligibility Check, Auto Retries, Manual Retry, Connector Agnostic MIT, External Vault, Platform Account, Business Profile Management, MCA Management, Organization Management, Merchant Account Management, Save Card Flow, Payment Sync, Void/Cancel, Payment Method Operations, Webhook Details, Browser Info, Customer Acceptance, Off Session Payments, Health Check, Payout Type, Payout Auto Fulfill, Dynamic Fields

### Notable gaps (not\_covered)
Dynamic Routing, 3DS Decision Rule, External 3DS Authentication, Payment Link, FRM Routing Algorithm, Tax Connector, Surcharge DSL, Reconciliation, OIDC Authentication, Relay Operations, Subscription Management

---

## Notes

### FRM Routing Algorithm (Bucket 3)
- Endpoint in CSV (`POST /routing (frm type)`) is **incorrect** — FRM routing is not managed by the routing service
- Actual endpoint: `POST /accounts/{merchant_id}` with `frm_routing_algorithm` field (merchant account update), or via business profile
- It selects which FRM connector (signifyd, riskified, etc.) handles fraud checks — not a routing DSL

### "configs table read" (Bucket 3 — 24 features)
- These features have no dedicated payment-time API endpoint
- They are configured as key-value pairs via `POST /configs` (create) or `POST /configs/{key}` (update) in the admin API
- The value is read internally during payment processing — the merchant cannot toggle them at payment time
- Examples: `requires_cvv`, `enable_extended_card_bin`, `should_call_gsm`, `implicit_customer_update`

---

## Coverage Gap Summary

| Bucket | Total rows | Covered | Gap (not\_covered + no\_config) |
|---|---|---|---|
| Bucket 1 | 98 | 8 (8%) | 90 (92%) |
| Bucket 2 | 858 | 312 (36%) | 546 (64%) |
| Bucket 3 | 99 | 28 (28%) | 71 (72%) |
| **Total** | **1055** | **348 (33%)** | **707 (67%)** |
