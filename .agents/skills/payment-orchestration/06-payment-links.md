---
name: hyperswitch-payment-links
description: Use this skill when the user asks about "payment links", "hosted checkout", "no-code payments", "shareable payment URL", "invoice link", "create a payment link", "payment link API", "custom payment page", "payment link branding", "POST /payment_links", or wants to collect payments without building a checkout UI.
version: 1.0.0
tags: [hyperswitch, payment-links, hosted-checkout, no-code]
---

# Payment Links

## Overview

Payment links are Hyperswitch-hosted checkout pages with a unique URL. Share them over email, SMS, WhatsApp, or embed them in invoices — customers click, pay, and you receive a webhook. No frontend code required.

## Prerequisites

- Hyperswitch API key
- `return_url` configured (optional — for post-payment redirect)
- Custom branding assets uploaded in dashboard (optional)

---

## Create a Payment Link

```bash
POST https://sandbox.hyperswitch.io/payment_links
{
  "amount": 25000,
  "currency": "USD",
  "description": "Invoice #INV-2024-042 — Consulting Services",
  "customer_id": "cus_abc123",
  "email": "client@example.com",
  "return_url": "https://yourapp.com/payment/complete",
  "expires_on": "2024-07-15T23:59:59.000Z",
  "metadata": { "invoice_id": "INV-2024-042", "project": "Q2-Retainer" },
  "payment_link_config": {
    "theme": "#1F2937",
    "logo": "https://yourapp.com/logo.png",
    "seller_name": "Acme Consulting LLC",
    "sdk_layout": "tabs"
  }
}
```

**Response:**
```json
{
  "payment_link_id": "plink_abc123",
  "payment_id": "pay_xyz789",
  "link": "https://pay.hyperswitch.io/payment_link/plink_abc123",
  "status": "active",
  "amount": 25000,
  "currency": "USD",
  "expires_on": "2024-07-15T23:59:59.000Z",
  "created": "2024-06-15T10:00:00.000Z"
}
```

Share `link` with your customer.

---

## Retrieve a Payment Link

```bash
GET /payment_links/{payment_link_id}
```

---

## List Payment Links

```bash
GET /payment_links/list?limit=20
```

Filter by `status`: `active`, `expired`, `payment_pending`, `payment_succeeded`, `invalidated`.

---

## Payment Link Config Options

| Field | Type | Description |
|-------|------|-------------|
| `theme` | hex color | Primary color of the checkout page (`#1A2B3C`) |
| `logo` | URL | Your logo displayed on the checkout page |
| `seller_name` | string | Company name shown on page |
| `sdk_layout` | enum | `tabs`, `accordion`, `spaced_accordion` |
| `background_image` | object | Background image URL and positioning |
| `display_sdk_only` | boolean | Show payment form without Hyperswitch branding |
| `enabled_saved_payment_method` | boolean | Show saved cards for returning customers |
| `hide_card_nickname_field` | boolean | Hide the card nickname input |

---

## Handle Payment Completion

When the customer completes payment:

1. They are redirected to your `return_url?payment_id=pay_xyz789`
2. **Always verify on the server** — do not trust the client-side redirect:

```javascript
app.get('/payment/complete', async (req, res) => {
  const { payment_id } = req.query;

  const payment = await hyperswitch.payments.retrieve(payment_id);

  if (payment.status === 'succeeded') {
    const invoiceId = payment.metadata.invoice_id;
    await invoices.markPaid(invoiceId, payment.payment_id);
    res.redirect(`/invoices/${invoiceId}/confirmed`);
  } else {
    res.redirect('/payment/failed');
  }
});
```

3. Also set up webhook for `payment.succeeded` — the redirect may not fire if the browser closes.

---

## Use Cases

### Invoice Collection

```json
{
  "amount": 150000,
  "currency": "USD",
  "description": "Invoice #INV-2024-099 — Software Development (June 2024)",
  "customer_id": "cus_client_007",
  "email": "finance@clientcompany.com",
  "expires_on": "2024-07-31T23:59:59.000Z",
  "metadata": { "invoice_id": "INV-2024-099", "due_date": "2024-07-31" }
}
```

### Event Registration

```json
{
  "amount": 9900,
  "currency": "USD",
  "description": "DevConf 2024 — Early Bird Ticket",
  "expires_on": "2024-08-01T00:00:00.000Z",
  "payment_link_config": { "seller_name": "DevConf 2024", "theme": "#6366F1" }
}
```

### Fixed-Price Product

```json
{
  "amount": 4999,
  "currency": "USD",
  "description": "Pro Plan — Monthly Subscription",
  "customer_id": "cus_new_user",
  "payment_link_config": {
    "theme": "#0EA5E9",
    "logo": "https://yourapp.com/assets/logo.png",
    "sdk_layout": "accordion"
  }
}
```

---

## Payment Link Lifecycle

```
active → payment_pending (customer is on the checkout page)
              ↓
         payment_succeeded (payment completed) → sends payment.succeeded webhook
         payment_failed    (payment declined)

active → expired (expires_on timestamp passed without payment)
active → invalidated (manually invalidated via API)
```

---

## Production Tips

- Set `expires_on` on all payment links — links without expiry create open liabilities. For invoices, align with your payment terms (net-30, net-60).
- Subscribe to `payment_link.payment_completed` and `payment_link.expired` webhooks to automate your invoice/order workflows.
- Payment links inherit your active routing configuration — a link to the same customer will route through your normal connector rules.
- For high-value B2B invoices, set `enabled_saved_payment_method: false` — business payers typically do not use saved personal cards.
- `payment_link_id` and `payment_id` are different — the link can be visited multiple times, but only one `payment_id` will be created. Check the link's `status` to know if it's been paid.
