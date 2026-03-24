---
name: list-payments
description: Use this skill when the user wants to "list payments", "filter transactions", "search payments", "get payment aggregates", "payment analytics", "reconcile payments", "export payment data", "paginate through payments", or needs to understand GET /payments/list, POST /payments/list, or GET /payments/aggregate.
version: 1.0.0
---

# List and Filter Payments

## When to Use

- Fetching a paginated list of payments for a merchant or customer
- Filtering by status, date range, amount, connector, or currency
- Getting aggregate counts and totals for analytics/dashboards
- Reconciling payments for a given period

## Key API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/payments/list` | List payments (simple query params) |
| POST | `/payments/list` | List with advanced filters (request body) |
| GET | `/payments/aggregate` | Get payment counts by status |
| GET | `/payments/filter` | Get available filter values for the UI |

## Query Parameters (GET /payments/list)

| Parameter | Type | Notes |
|-----------|------|-------|
| `limit` | integer | Max records per page (default: 10, max: 100) |
| `offset` | integer | Pagination offset |
| `time_range.start_time` | string | ISO 8601, e.g. `"2024-01-01T00:00:00Z"` |
| `time_range.end_time` | string | ISO 8601 |
| `status` | enum | `succeeded`, `failed`, `processing`, `cancelled`, etc. |
| `currency` | string | ISO 4217 |
| `customer_id` | string | Filter by customer |
| `connector` | string | e.g. `"stripe"`, `"adyen"` |

## Common Scenarios

### 1. List Recent Payments

```
GET /payments/list?limit=20&time_range.start_time=2024-01-01T00:00:00Z
```

### 2. List Failed Payments in a Date Range

```
GET /payments/list?status=failed&time_range.start_time=2024-06-01T00:00:00Z&time_range.end_time=2024-06-30T23:59:59Z&limit=50
```

### 3. Advanced Filter with POST

```json
POST /payments/list
{
  "time_range": {
    "start_time": "2024-01-01T00:00:00Z",
    "end_time": "2024-12-31T23:59:59Z"
  },
  "status": ["succeeded", "partially_captured"],
  "currency": ["USD", "EUR"],
  "connector": ["stripe"],
  "limit": 100,
  "offset": 0
}
```

### 4. Payment Aggregates (Dashboard KPIs)

```
GET /payments/aggregate?time_range.start_time=2024-01-01T00:00:00Z
```
Returns counts grouped by status:
```json
{
  "status_with_count": [
    { "status": "succeeded", "count": 1423 },
    { "status": "failed", "count": 87 },
    { "status": "processing", "count": 12 }
  ]
}
```

### 5. Paginate Through All Results

```python
offset = 0
limit = 100
while True:
    resp = GET /payments/list?limit={limit}&offset={offset}
    results.extend(resp.data)
    if len(resp.data) < limit:
        break
    offset += limit
```

## Response Structure

```json
{
  "count": 20,
  "total_count": 1523,
  "data": [
    {
      "payment_id": "pay_abc123",
      "amount": 1000,
      "currency": "USD",
      "status": "succeeded",
      "created": "2024-06-15T10:30:00Z",
      ...
    }
  ]
}
```

## Tips & Gotchas

- Default `limit` is 10; always set it explicitly to avoid unexpected truncation.
- `total_count` in the response lets you calculate total pages without separate aggregate calls.
- Date filters use `time_range.start_time` / `time_range.end_time` — not `from` / `to`. Wrong field names silently return unfiltered results.
- The POST `/payments/list` form accepts arrays for `status` and `currency`, enabling multi-value filters.
- For large exports (>10k records), batch by day rather than requesting everything at once to avoid timeouts.
- `GET /payments/aggregate` is cheaper than listing and counting — prefer it for dashboard KPIs.
