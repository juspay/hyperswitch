---
name: hyperswitch-refunds-disputes
description: Use this skill when the user wants to "refund a payment", "issue a refund", "partial refund", "full refund", "reverse a charge", "handle a chargeback", "respond to a dispute", "dispute evidence", "check refund status", "why is my refund pending", or needs to understand POST /refunds, GET /refunds/{id}, or dispute management in Hyperswitch.
version: 1.0.0
tags: [hyperswitch, refunds, disputes, chargebacks]
---

# Refunds & Disputes

## Overview

Refunds return captured funds to a customer — fully or partially. Disputes (chargebacks) are initiated by the cardholder's bank and require evidence submission. This skill covers the complete lifecycle of both flows.

## Prerequisites

- Payment must be in `succeeded` status to be refunded
- Dispute management requires webhook handling (disputes arrive asynchronously)

---

## Part 1: Refunds

### Create a Full Refund

```bash
POST https://sandbox.hyperswitch.io/refunds
{
  "payment_id": "pay_abc123",
  "reason": "customer_request"
}
```

Omitting `amount` refunds the full captured amount.

**Response:**
```json
{
  "refund_id": "ref_xyz789",
  "payment_id": "pay_abc123",
  "amount": 5000,
  "currency": "USD",
  "status": "pending",
  "reason": "customer_request",
  "created_at": "2024-06-15T11:00:00.000Z"
}
```

Refunds start as `pending` and transition asynchronously via webhooks.

---

### Create a Partial Refund

```bash
POST /refunds
{
  "payment_id": "pay_abc123",
  "amount": 2000,
  "reason": "return",
  "metadata": { "returned_sku": "ITEM-001", "return_id": "RET-2024-005" }
}
```

---

### Multiple Partial Refunds

You can issue multiple partial refunds against the same payment as long as the cumulative total does not exceed the captured amount:

```bash
# Refund 1: $20.00
POST /refunds
{ "payment_id": "pay_abc123", "amount": 2000, "reason": "return" }

# Refund 2: $10.00 (total $30 of, say, $50 captured)
POST /refunds
{ "payment_id": "pay_abc123", "amount": 1000, "reason": "return" }
```

---

### Retrieve a Refund

```bash
GET /refunds/{refund_id}
```

---

### List Refunds for a Payment

```bash
POST /refunds/list
{
  "payment_id": "pay_abc123"
}
```

---

### Refund Status Lifecycle

```
pending
   │
   ├──→ succeeded   (funds returned; typically 5–10 business days on card)
   ├──→ failed      (connector rejected — check refund again or contact support)
   └──→ review      (flagged for manual review by risk system)
```

**Webhook events:**
- `refund.succeeded` — update your order management system
- `refund.failed` — notify ops team; may need manual intervention

---

### Refund `reason` Values

| Value | When to Use |
|-------|-------------|
| `customer_request` | Customer asked for refund |
| `duplicate` | Payment was charged twice |
| `fraudulent` | Confirmed fraud — use carefully; affects risk signals |
| `return` | Physical goods returned |
| `other` | None of the above |

---

### Idempotency

Always pass an `Idempotency-Key` header on refund creation to safely retry network failures:

```bash
curl -H "Idempotency-Key: ref-retry-550e8400-e29b" \
     -H "api-key: YOUR_KEY" \
     POST /refunds ...
```

Duplicate requests with the same key return the original refund without double-refunding.

---

## Part 2: Disputes (Chargebacks)

A dispute is opened by the cardholder's bank. You receive a webhook event and must respond with evidence before the deadline.

### Dispute Webhook Event

```json
{
  "type": "dispute.opened",
  "data": {
    "dispute_id": "dp_abc123",
    "payment_id": "pay_xyz789",
    "amount": 5000,
    "currency": "USD",
    "dispute_stage": "dispute",
    "dispute_status": "dispute_opened",
    "connector_dispute_id": "ch_dispute_stripe_001",
    "challenge_required_by": "2024-07-01T00:00:00.000Z",
    "reason": "product_not_received"
  }
}
```

**Dispute stages:** `pre_dispute` → `dispute` → `pre_arbitration`

---

### List Open Disputes

```bash
GET /disputes/list?dispute_status=dispute_opened
```

---

### Accept a Dispute

If you cannot or choose not to contest:

```bash
POST /disputes/{dispute_id}/accept
```

---

### Submit Dispute Evidence

Gather and submit evidence before `challenge_required_by`:

```bash
POST /disputes/{dispute_id}/evidence
{
  "cancellation_policy": "base64_encoded_pdf_or_url",
  "customer_communication": "base64_or_url",
  "shipping_documentation": "base64_or_url",
  "customer_email_address": "customer@example.com",
  "customer_name": "Jane Smith",
  "product_description": "Pro subscription plan — digital service, no physical delivery",
  "shipping_date": "2024-06-01T00:00:00Z",
  "service_date": "2024-06-01T00:00:00Z",
  "access_activity_log": "User logged in on 2024-06-01 and accessed plan features..."
}
```

---

### Dispute Evidence Fields

| Field | Use When |
|-------|---------|
| `cancellation_policy` | Subscription / service dispute |
| `customer_communication` | Customer confirmed receipt via email/chat |
| `shipping_documentation` | Physical goods — tracking number, carrier proof |
| `service_documentation` | Digital service — access logs, screenshots |
| `refund_policy` | Dispute about expected refund |
| `duplicate_charge_documentation` | Duplicate charge dispute |

---

### Dispute Webhook Events

| Event | Action Required |
|-------|----------------|
| `dispute.opened` | Log dispute, start collecting evidence |
| `dispute.expired` | Deadline passed — dispute resolved against you |
| `dispute.accepted` | You accepted the dispute |
| `dispute.won` | Evidence accepted — funds returned to you |
| `dispute.lost` | Dispute resolved against you |

---

## Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `422 refund_amount_exceeds_payment` | Refund total > captured amount | Check existing refunds against the payment first |
| `422 payment_not_succeeded` | Trying to refund a non-captured payment | Only refund `succeeded` payments |
| `404 dispute_not_found` | Wrong `dispute_id` | Verify from webhook payload |
| `422 dispute_challenge_expired` | Missed the evidence deadline | Accept the dispute; escalate to your PSP |

---

## Production Tips

- Set up webhooks for both `refund.succeeded` and `refund.failed` — refunds are async and you should not poll.
- Store `refund_id` in your database immediately after `POST /refunds` returns `pending` — you need it for tracking.
- For fraud disputes, include as much digital evidence as possible: IP logs, device fingerprints, login history, delivery confirmation.
- Refunds on SEPA/ACH take longer (3–5 days longer than card refunds) — set customer expectations accordingly.
- `reason: "fraudulent"` sends signals to the connector's risk engine — only use for confirmed fraud cases.
