# Threshold override cutover

This cutover moves only `hyperswitch_alerts_sr_thresholds`. Analytical ClickHouse reads and all
other alert state remain unchanged. There is no runtime dual write.

## Preconditions

1. Deploy the observability migration, but do not switch the hyperswitch-alerts client yet.
2. Pause threshold writes in hyperswitch-alerts for the duration of the export and import.
3. Configure `clickhouse-client` (its normal config file is preferred, so credentials do not enter
   shell history or process arguments) for the source cluster.
4. Export `OBSERVABILITY_DATABASE_URL` for the destination PostgreSQL database.
5. Confirm `success_rate_threshold_overrides` is empty. The script also enforces this.

## Backfill

Run from the repository root:

```bash
scripts/backfill_threshold_overrides.sh
```

The script reads `hyperswitch_alerts_sr_thresholds FINAL`, which asks the source
`ReplacingMergeTree` to collapse each natural key before deleted rows are filtered. It imports all
active names and products, including zero-success configurations, and preserves nullable values,
actor, and update time. Merchant and profile identifiers are trimmed to the API's storage contract.
The import is one PostgreSQL transaction and aborts for invalid keys, invalid actors, or collisions
introduced by trimming. It refuses a non-empty destination so it cannot overwrite post-cutover
writes.

A successful run prints equal source and destination active-row counts. Before proceeding, also
spot-check representative merchant-wide and profile-specific rows:

```sql
SELECT name, product, merchant_id, profile_id,
       min_volume, min_impacted_volume, tolerance, diff_threshold,
       updated_by, last_updated_at
FROM success_rate_threshold_overrides
WHERE NOT is_deleted
ORDER BY merchant_id, profile_id, name, product
LIMIT 20;
```

## Switch and rollback

After verification, configure hyperswitch-alerts with the observability-plane base URL and internal
API key, then resume threshold writes. Do not enable a ClickHouse threshold dual write.

If verification fails before the client switch, keep hyperswitch-alerts on ClickHouse, clear the
PostgreSQL threshold table, correct the source/configuration issue, and rerun. After the client
switch, PostgreSQL is authoritative; do not roll back the reader independently because new writes
would be lost.
