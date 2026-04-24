# Production Usage Queries

Run these queries against production ClickHouse, export the results as CSV,
and share them back. The import script will merge the data into the three
bucket CSVs as two new columns: `prod_used` and `latest_prod_timestamp`.

## Files

| File            | Output columns                              | Expected CSV               |
|-----------------|---------------------------------------------|----------------------------|
| `bucket_1.sql`  | `feature, connector, call_count, last_seen` | `bucket_1_results.csv`     |
| `bucket_2.sql`  | `feature, connector, pm, pmt, call_count, last_seen` | `bucket_2_results.csv` |
| `bucket_3.sql`  | `feature, call_count, last_seen`            | `bucket_3_results.csv`     |

## How to run

Each file is a single `SELECT ... UNION ALL ...` statement. You can:

- **Run all at once**: paste the full file into your ClickHouse client and execute
- **Run one feature at a time**: copy out a single `SELECT` block — each sub-query
  is self-contained with its own `WHERE` clause and `GROUP BY`

All timestamps are wrapped in `toString()` to avoid type-mismatch errors
across tables using different `DateTime` precisions.

## Lookback window

Default is `INTERVAL 90 DAY` in every sub-query. Search-and-replace to change.

## Feature coverage

| Bucket | Total features | Queried here | Undetectable from events |
|--------|----------------|--------------|---------------------------|
| 1      | 27             | 18           | 9                         |
| 2      | 3              | 2            | 1 (Decrypt Flow)          |
| 3      | 99             | ~35          | ~65 (config-level)        |

Features that cannot be detected from events (configuration-level flags,
request-JSON-only fields, etc.) are listed in the SQL file headers. Those
rows will be marked `prod_used = unknown` in the output CSVs.

## Import into bucket CSVs

Once you share the 3 result CSVs back, the import script (to be written)
will add two columns to each bucket CSV:

- `prod_used`: `yes` / `no` / `unknown`
- `latest_prod_timestamp`: ISO timestamp of last production occurrence

Matching keys:
- Bucket 1: `(connector, feature)`
- Bucket 2: `(connector, pm, pmt, feature)`
- Bucket 3: `(feature)`
