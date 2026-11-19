---
name: hyperswitch-webhooks
description: Use this skill when the user asks about "webhook handling", "payment webhooks", "event processing", "webhook signature verification", "HMAC verification", "payment.succeeded event", "refund webhook", "dispute webhook", "async payment notification", "how do I know when a payment succeeds", "webhook retry", or needs to build a reliable webhook handler for Hyperswitch.
version: 1.0.0
tags: [hyperswitch, webhooks, events, async, signature-verification]
---

# Webhook Handling

## Overview

Hyperswitch uses webhooks to notify your server of async payment events — payment outcomes, refund completions, dispute openings. A reliable webhook handler is critical for triggering fulfillment, updating order state, and avoiding race conditions between polling and events.

## Prerequisites

- A publicly accessible HTTPS endpoint (not `localhost`) — use [smee.io](https://smee.io) or [ngrok](https://ngrok.com) for local development
- Webhook URL registered in **Developers → Webhooks** in the Hyperswitch dashboard
- Webhook signing secret (shown once at registration — store it securely)

---

## Step 1: Register Your Webhook

In the dashboard:
1. **Developers → Webhooks → Add Endpoint**
2. Enter your HTTPS URL (e.g., `https://api.yourapp.com/webhooks/hyperswitch`)
3. Select events to subscribe to (or subscribe to all)
4. Copy the **signing secret** — you'll need it for verification

---

## Step 2: Verify the Webhook Signature

**Never process a webhook without verifying its signature.** Hyperswitch signs every request with HMAC-SHA256.

The signature is in the `x-webhook-signature-512` header.

### Node.js (Express)

```javascript
const crypto = require('crypto');
const express = require('express');
const app = express();

// Raw body is required for signature verification — do NOT use express.json() before this route
app.post('/webhooks/hyperswitch',
  express.raw({ type: 'application/json' }),
  (req, res) => {
    const signature = req.headers['x-webhook-signature-512'];
    const secret = process.env.HYPERSWITCH_WEBHOOK_SECRET;

    const expectedSig = crypto
      .createHmac('sha512', secret)
      .update(req.body)
      .digest('hex');

    if (signature !== expectedSig) {
      console.error('Webhook signature mismatch');
      return res.status(401).send('Unauthorized');
    }

    const event = JSON.parse(req.body);
    handleWebhookEvent(event);
    res.status(200).send('OK');
  }
);
```

### Python (Flask)

```python
import hmac
import hashlib
import os
from flask import Flask, request, abort

app = Flask(__name__)

@app.route('/webhooks/hyperswitch', methods=['POST'])
def webhook():
    signature = request.headers.get('x-webhook-signature-512')
    secret = os.environ['HYPERSWITCH_WEBHOOK_SECRET'].encode()

    expected = hmac.new(secret, request.data, hashlib.sha512).hexdigest()

    if not hmac.compare_digest(signature, expected):
        abort(401)

    event = request.get_json()
    handle_event(event)
    return '', 200
```

> Always use a **constant-time comparison** (`hmac.compare_digest` in Python, timing-safe comparison in other languages) to prevent timing attacks.

---

## Step 3: Handle Events

### Event Structure

```json
{
  "type": "payment.succeeded",
  "event_id": "evt_01HX...",
  "created": "2024-06-15T10:31:00.000Z",
  "object_id": "pay_abc123",
  "object_type": "payment",
  "content": {
    "type": "payment_details",
    "object": {
      "payment_id": "pay_abc123",
      "amount": 5000,
      "currency": "USD",
      "status": "succeeded",
      "metadata": { "order_id": "ORD-001" },
      ...
    }
  }
}
```

---

### Event Reference

| Event | Trigger | Action |
|-------|---------|--------|
| `payment.succeeded` | Payment captured | Trigger fulfillment, send receipt |
| `payment.failed` | Authorization or capture failed | Notify customer, retry or abandon |
| `payment.processing` | Payment is being processed | Update UI to "pending" |
| `payment.cancelled` | Payment voided | Release reserved inventory |
| `payment.requires_customer_action` | 3DS challenge needed | Redirect customer |
| `refund.succeeded` | Refund settled | Update order state, notify customer |
| `refund.failed` | Refund rejected by connector | Alert ops team |
| `dispute.opened` | Chargeback filed | Begin evidence collection |
| `dispute.won` | Dispute resolved in your favor | Log outcome |
| `dispute.lost` | Dispute resolved against you | Log outcome, review fraud controls |

---

### Handler Implementation Pattern

```javascript
function handleWebhookEvent(event) {
  // Idempotency: check if this event_id was already processed
  if (db.eventAlreadyProcessed(event.event_id)) {
    console.log(`Duplicate event ${event.event_id}, skipping`);
    return;
  }

  switch (event.type) {
    case 'payment.succeeded': {
      const payment = event.content.object;
      const orderId = payment.metadata?.order_id;
      orders.fulfill(orderId, payment.payment_id);
      notifications.sendReceipt(payment.customer_id, payment.amount, payment.currency);
      break;
    }
    case 'payment.failed': {
      const payment = event.content.object;
      notifications.notifyPaymentFailed(payment.customer_id, payment.error_message);
      break;
    }
    case 'refund.succeeded': {
      const refund = event.content.object;
      orders.markRefunded(refund.payment_id, refund.refund_id, refund.amount);
      break;
    }
    case 'dispute.opened': {
      const dispute = event.content.object;
      disputes.openTicket(dispute.dispute_id, dispute.payment_id, dispute.challenge_required_by);
      break;
    }
    default:
      console.log(`Unhandled event type: ${event.type}`);
  }

  db.markEventProcessed(event.event_id);
}
```

---

## Retry Behavior

Hyperswitch retries failed webhook deliveries with exponential backoff:

| Attempt | Delay |
|---------|-------|
| 1 | Immediate |
| 2 | 5 minutes |
| 3 | 30 minutes |
| 4 | 2 hours |
| 5 | 8 hours |

A delivery is considered **failed** if your endpoint returns a non-2xx status or times out (>30 seconds).

---

## Testing Webhooks Locally

```bash
# Install smee client
npm install -g smee-client

# Forward smee.io channel to local server
smee --url https://smee.io/your-channel --path /webhooks/hyperswitch --port 3000
```

Register `https://smee.io/your-channel` as your webhook URL in the dashboard during development.

---

## Production Tips

- **Respond fast, process async**: Return `200 OK` immediately. Process the event in a background job/queue. Slow handlers cause retry storms.
- **Idempotency is mandatory**: Webhooks can be delivered more than once. Use `event_id` as a deduplication key in your database.
- **Never trust webhook data alone for fulfillment**: After receiving `payment.succeeded`, call `GET /payments/{id}` to confirm the status before fulfilling — this prevents spoofed webhooks from triggering free fulfillment.
- **Set up dead-letter handling**: Log events that fail all retries to a queue for manual review.
- **Raw body required**: Signature verification requires the exact raw bytes. Parsing JSON before verifying breaks the HMAC. Express's `json()` middleware must not run before the raw body is read.
