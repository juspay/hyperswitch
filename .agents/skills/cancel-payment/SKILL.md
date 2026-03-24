---
name: cancel-payment
description: Use this skill when the user wants to "cancel a payment", "void a payment", "abort a transaction", "cancel before capture", "cancel post capture", "cancel_post_capture", or needs to understand POST /payments/{id}/cancel or POST /payments/{id}/cancel_post_capture. Covers which payment states allow cancellation and the difference between pre-capture and post-capture voids.
version: 1.0.0
---

# Cancel a Payment

## When to Use

- Voiding a payment before it is captured (pre-capture cancel)
- Cancelling a captured payment before it settles (post-capture void)
- Releasing a hold on a customer's funds
- Abandoning a payment in `requires_confirmation` or `requires_payment_method` state

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/payments/{payment_id}/cancel` | Cancel before capture (void authorization) |
| POST | `/payments/{payment_id}/cancel_post_capture` | Cancel after capture, before settlement |

## Essential Fields

### Cancel (pre-capture)
| Field | Type | Notes |
|-------|------|-------|
| `cancellation_reason` | enum | `duplicate`, `fraudulent`, `requested_by_customer`, `abandoned`, `void` |

### Cancel Post Capture
No required body fields — the endpoint itself signals the intent.

## Cancellable Payment States

```
requires_payment_method  ✓ cancellable
requires_confirmation    ✓ cancellable
requires_capture         ✓ cancellable (voids authorization)
processing               ✓ may be cancellable (connector-dependent)
succeeded                ✗ NOT cancellable → use refund instead
failed                   ✗ already terminal
cancelled                ✗ already terminal
```

## Common Scenarios

### 1. Cancel an Authorization (pre-capture)

```json
POST /payments/{payment_id}/cancel
{
  "cancellation_reason": "requested_by_customer"
}
```
Releases the hold on the customer's card. Payment moves to `cancelled`.

### 2. Cancel an Abandoned Cart

```json
POST /payments/{payment_id}/cancel
{
  "cancellation_reason": "abandoned"
}
```
Use for payments in `requires_confirmation` that the user never completed.

### 3. Cancel Post Capture (before settlement)

```json
POST /payments/{payment_id}/cancel_post_capture
{}
```
Voids a capture before it settles with the card network. Window is very narrow (minutes to hours depending on connector).

### 4. Check if Cancellation is Possible First

```json
GET /payments/{payment_id}
// Check response.status before attempting cancel
```

## Tips & Gotchas

- **Succeeded payments cannot be cancelled** — issue a refund (`POST /refunds`) instead.
- Post-capture cancellation (`cancel_post_capture`) has a very short window and is connector-dependent. Most connectors do not support it after settlement begins.
- A voided authorization releases the held funds back to the customer within 1-7 business days (card network dependent).
- `cancellation_reason` affects risk signals — use `fraudulent` only when you have genuine fraud indicators.
- Cancelling a payment in `requires_payment_method` (e.g., created but no card attached) is effectively just cleanup.
- If a payment is stuck in `processing`, cancellation may return an error — retry after a short delay or wait for the payment to reach a terminal state.
