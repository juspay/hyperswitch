---
name: refund-payment
description: Use this skill when the user wants to "refund a payment", "reverse a charge", "process a refund", "partial refund", "full refund", "issue a refund to a customer", "check refund status", or needs to understand POST /refunds, GET /refunds/{id}, or POST /refunds/list. Covers full/partial refunds, refund reasons, idempotency, and status polling.
version: 1.0.0
---

# Refund a Payment

## When to Use

- Returning funds to a customer (full or partial)
- Issuing multiple partial refunds against a single payment
- Checking the status of a refund
- Listing refunds for reconciliation

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/refunds` | Create a refund |
| GET | `/refunds/{refund_id}` | Retrieve refund status |
| POST | `/refunds/{refund_id}` | Update refund metadata |
| POST | `/refunds/list` | List refunds with filters |

## Essential Fields

| Field | Type | Notes |
|-------|------|-------|
| `payment_id` | string | The payment to refund (required) |
| `amount` | integer | In smallest currency unit; omit for full refund |
| `reason` | enum | `customer_request`, `fraud`, `return`, `duplicate`, `other` |
| `refund_type` | enum | `instant` or `scheduled` |
| `metadata` | object | Arbitrary key-value pairs |

## Common Scenarios

### 1. Full Refund

```json
POST /refunds
{
  "payment_id": "pay_abc123",
  "reason": "customer_request"
}
```
Refunds the entire captured amount. Omitting `amount` triggers a full refund.

### 2. Partial Refund

```json
POST /refunds
{
  "payment_id": "pay_abc123",
  "amount": 500,
  "reason": "return",
  "metadata": { "returned_items": "SKU-001" }
}
```

### 3. Multiple Partial Refunds

```json
// First refund: $5.00
POST /refunds
{ "payment_id": "pay_abc123", "amount": 500, "reason": "return" }

// Second refund: $3.00
POST /refunds
{ "payment_id": "pay_abc123", "amount": 300, "reason": "return" }
```
Total refunded cannot exceed the captured amount.

### 4. Check Refund Status

```json
GET /refunds/{refund_id}
```
Response `status` will be one of: `pending`, `succeeded`, `failed`, `review`.

### 5. List Refunds for a Payment

```json
POST /refunds/list
{
  "payment_id": "pay_abc123"
}
```

## Refund Status Flow

```
pending → succeeded  (funds returned, typically 5-10 business days)
        → failed     (connector rejected — check refund reason / contact support)
        → review     (flagged for manual review)
```

## Tips & Gotchas

- A payment must be in `succeeded` status to be refunded. Attempting to refund `requires_capture` or `failed` payments returns a 422.
- Refunds are **asynchronous** — `POST /refunds` returns `pending`. Poll `GET /refunds/{id}` or use webhooks (`refund.succeeded`, `refund.failed`).
- The sum of all refunds on a payment cannot exceed the captured amount.
- `instant` refunds are connector-dependent — not all processors support them.
- Use an `Idempotency-Key` header to safely retry refund creation without issuing duplicate refunds.
- Refunds cannot be cancelled once initiated.
- `fraud` reason triggers additional risk signals in some connectors — use it accurately.
