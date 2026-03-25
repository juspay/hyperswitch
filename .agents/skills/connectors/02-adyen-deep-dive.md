---
name: hyperswitch-adyen-connector
description: Use this skill when the user asks about "Adyen connector in Hyperswitch", "Adyen API key setup", "Adyen merchant account", "Adyen HMAC webhook", "Adyen test cards", "Adyen local payment methods", "Adyen 3DS", "configure Adyen in Hyperswitch", "Adyen HPP", or encounters Adyen-specific errors when using Hyperswitch.
version: 1.0.0
tags: [hyperswitch, adyen, connector, integration, europe, apac]
---

# Adyen Connector Deep Dive

## Overview

Adyen is a tier-1 global payment processor with exceptional coverage across Europe, APAC, and LATAM. It supports 200+ payment methods locally. This guide covers Adyen-specific setup, HMAC webhook configuration, supported payment methods, test cards, and known quirks within Hyperswitch.

## Prerequisites

- Adyen Customer Area access ([ca-test.adyen.com](https://ca-test.adyen.com) for sandbox)
- A Merchant Account name (not your Adyen email — a dedicated merchant account identifier)
- API key with required permissions

---

## Step 1: Create an API Key in Adyen

1. **Adyen Customer Area → Developers → API credentials**
2. Click **Create new credential** → choose **API key**
3. Assign roles: `Checkout webservice role`, `Merchant PAL webservice role`
4. Copy the **API key** (format: `AQE...`)

---

## Step 2: Get Your Merchant Account Name

Your Merchant Account is a string identifier (e.g., `YourCompanyECOM`), visible in:
- Top-right of the Adyen Customer Area
- **Account → Merchant accounts**

This is different from your company name or email.

---

## Step 3: Configure in Hyperswitch

In **Hyperswitch Dashboard → Connectors → Adyen**:

| Field | Value |
|-------|-------|
| API Key | `AQE...` (your Adyen API key) |
| Merchant Account | `YourCompanyECOM` (exact string from Adyen CA) |
| HMAC Key | (generated in Step 4) |
| Mode | Test |

---

## Step 4: Configure Adyen Webhook → Hyperswitch (HMAC)

Adyen uses HMAC-SHA256 for webhook verification:

1. **Adyen Customer Area → Developers → Webhooks → Add new webhook** → Standard Notification
2. URL: `https://sandbox.hyperswitch.io/webhooks/{your_merchant_id}/adyen`
3. **Additional settings → HMAC key** → Generate → copy the HMAC key
4. Enter this HMAC key in Hyperswitch connector configuration
5. Select active events: `AUTHORISATION`, `REFUND`, `CHARGEBACK`, `CANCELLATION`, `CAPTURE`
6. Set **SSL version** to TLS 1.2
7. Click **Test Configuration** to verify

---

## Supported Payment Methods via Adyen

| Payment Method | Hyperswitch Config | Countries |
|---------------|-------------------|-----------|
| Credit/Debit Card | `card` | Global |
| Apple Pay | `wallet → apple_pay` | iOS Safari |
| Google Pay | `wallet → google_pay` | Android/Chrome |
| iDEAL | `bank_redirect → ideal` | Netherlands |
| SOFORT | `bank_redirect → sofort` | DE, AT, CH, NL, BE |
| Giropay | `bank_redirect → giropay` | Germany |
| EPS | `bank_redirect → eps` | Austria |
| Bancontact | `bank_redirect → bancontact` | Belgium |
| BLIK | `bank_redirect → blik` | Poland |
| Klarna | `pay_later → klarna` | EU, US, AU |
| PayPal | `wallet → paypal_redirect` | Global |
| SEPA Direct Debit | `bank_debit → sepa` | Eurozone |
| WeChat Pay | `wallet → wechat_pay` | China |
| Alipay | `wallet → ali_pay` | China |
| UPI | (requires Adyen India activation) | India |
| Multibanco | `bank_redirect → multibanco` | Portugal |
| MB WAY | `bank_redirect → mb_way` | Portugal |
| Vipps | `bank_redirect → vipps` | Norway |
| Twint | `bank_redirect → twint` | Switzerland |

---

## Adyen Test Cards

| Card Number | Network | Scenario |
|-------------|---------|----------|
| `4111111111111111` | Visa | Authorized |
| `5500000000000004` | Mastercard | Authorized |
| `370000000000002` | Amex | Authorized |
| `4988438843884305` | Visa | Refused (declined) |
| `4166676667666746` | Visa | 3DS challenge |
| `4000000000001091` | Visa | 3DS frictionless |
| `5100000000000511` | Mastercard | Refused (blocked) |

Expiry: use any future date. CVC: any 3 digits.

**Test iDEAL:** Adyen sandbox iDEAL always redirects to a mock bank page. Select any bank, click **Authorize**.

---

## 3DS Behavior with Adyen

Adyen implements 3DS2 natively. When `authentication_type: "three_ds"` is set:

- Adyen evaluates whether a challenge is required based on the transaction risk score
- Frictionless: Adyen authenticates in the background — no redirect
- Challenge: Customer is redirected to the issuer ACS

Adyen-specific 3DS config in Hyperswitch:
```json
{
  "authentication_type": "three_ds",
  "browser_info": {
    "user_agent": "...",
    "accept_header": "text/html,application/xhtml+xml",
    "language": "en-US",
    "color_depth": 24,
    "screen_height": 900,
    "screen_width": 1440,
    "time_zone": -480,
    "java_enabled": false,
    "java_script_enabled": true
  }
}
```

For stronger 3DS data quality (better frictionless rates), pass `browser_info` on every payment.

---

## Adyen-Specific Error Codes

| Adyen resultCode | Meaning | Action |
|-----------------|---------|--------|
| `Refused` | Hard decline | Do not retry same card |
| `Error` | Technical error | Retry once |
| `Cancelled` | Customer cancelled | Re-present checkout |
| `Pending` | Async payment (bank debit, voucher) | Wait for webhook |
| `AuthenticationNotRequired` | 3DS exemption granted | Payment proceeds |
| `AuthenticationFinished` | 3DS completed (frictionless or challenge) | Payment proceeds |

---

## Known Adyen Quirks in Hyperswitch

1. **Merchant Account is case-sensitive**: `YourCompanyECOM` ≠ `yourcompanyecom`. Exact string match required.

2. **HMAC key rotation**: If you regenerate the HMAC key in Adyen, update it in Hyperswitch **before** the old key expires. A mismatch causes all Adyen webhooks to fail.

3. **Capture delay**: Adyen has a configurable capture delay (default: immediate for card-present, 24h for e-commerce in some regions). For `capture_method: manual` in Hyperswitch, ensure Adyen's `captureDelay` is set to `manual` in your merchant account settings in Adyen CA.

4. **APM redirect timeouts**: For iDEAL, Giropay, and other redirect-based APMs, Adyen has a 30-minute session timeout. If a customer takes longer, the payment transitions to `expired`. Handle `payment.processing` → `payment.failed` webhooks for this case.

5. **Refund lag**: Adyen refunds are queued and processed in batches. The `refund.succeeded` webhook may arrive 24–48 hours after `POST /refunds`, even in sandbox.

---

## Debugging Adyen Payments

1. Get `connector_transaction_id` from Hyperswitch payment response
2. In Adyen Customer Area → **Transactions → Payments** → search by the PSP reference (= `connector_transaction_id`)
3. Click the transaction → **Event log** shows every status transition with reason codes

---

## Production Tips

- For European payments, configure **Adyen's 3DS2 exemptions** in your merchant account settings to maximize frictionless rates (reduces friction while maintaining SCA compliance).
- Adyen has strong local payment method coverage — enable local methods for each geography you serve. A German customer offered SOFORT converts significantly better than card-only.
- Use **Adyen's risk settings** (CA → Risk → Risk profiles) to tune fraud rules per merchant account rather than relying solely on Hyperswitch routing for fraud management.
- Adyen's `merchantOrderReference` maps to Hyperswitch's `merchant_order_reference_id` — set this to your internal order ID for easy cross-reference in the Adyen CA.
