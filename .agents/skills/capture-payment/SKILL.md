---
name: capture-payment
description: Use this skill when the user wants to "capture a payment", "capture authorized funds", "finalize an authorization", "partially capture", "incremental authorization", "increase authorized amount", or needs to understand POST /payments/{id}/capture or POST /payments/{id}/incremental_authorization. Applies after a payment is in requires_capture state.
version: 1.0.0
---

# Capture a Payment

## When to Use

- Finalizing a payment that was created with `capture_method: manual`
- Capturing less than the authorized amount (partial capture)
- Increasing the authorized amount before capture (incremental auth — hotels, car rentals)
- Extending an authorization window

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/payments/{payment_id}/capture` | Capture authorized funds (full or partial) |
| POST | `/payments/{payment_id}/incremental_authorization` | Increase authorized amount before capture |
| POST | `/payments/{payment_id}/extend_authorization` | Extend the authorization validity window |
| POST | `/payments/{payment_id}/cancel_post_capture` | Void a capture that hasn't settled |

## Prerequisites

Payment must be in `requires_capture` status. This means it was created with `capture_method: "manual"` and confirmed successfully.

## Essential Fields

### Capture
| Field | Type | Notes |
|-------|------|-------|
| `amount_to_capture` | integer | Optional — omit to capture full authorized amount |
| `statement_descriptor_suffix` | string | Optional suffix on bank statement |

### Incremental Authorization
| Field | Type | Notes |
|-------|------|-------|
| `amount` | integer | New **total** amount (not the delta) |
| `reason` | string | Reason for increase, e.g. `"additional_services"` |

## Common Scenarios

### 1. Full Capture

```json
POST /payments/{payment_id}/capture
{}
```
Captures the full authorized amount. Payment moves to `succeeded`.

### 2. Partial Capture

```json
POST /payments/{payment_id}/capture
{
  "amount_to_capture": 800
}
```
If authorized for $20.00, capture $8.00. Remaining $12.00 is released. Payment moves to `partially_captured` then `succeeded`.

### 3. Incremental Authorization (Hotels / Car Rentals)

```json
// Original authorization: $150
POST /payments/{payment_id}/incremental_authorization
{
  "amount": 200,
  "reason": "room_service_charges"
}
// Authorization is now $200. Then capture:
POST /payments/{payment_id}/capture
{}
```

### 4. Extend Authorization Window

```json
POST /payments/{payment_id}/extend_authorization
{
  "amount": 5000
}
```
Resets the authorization expiry clock. Useful when fulfillment is delayed.

## Payment Status Flow

```
requires_capture
      │
      ├─[capture full]──────────→ succeeded
      ├─[capture partial]───────→ partially_captured_and_capturable → succeeded
      └─[cancel]────────────────→ cancelled
```

## Tips & Gotchas

- Capture must happen within the authorization window (typically 7 days for cards, varies by connector).
- `amount_to_capture` must be ≤ the authorized amount. Trying to capture more returns a 422.
- Not all connectors support incremental authorization — check connector docs before relying on it.
- Partial capture behavior varies: some connectors auto-release the remainder, others require explicit cancel.
- After capture, use `POST /payments/{id}/cancel_post_capture` to void (only before settlement).
- Pass `Idempotency-Key` header on capture calls — double-capture on the same payment can cause issues.
