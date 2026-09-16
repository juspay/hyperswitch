#!/usr/bin/env bash
# One-shot cutover backfill for success-rate threshold overrides.
set -euo pipefail

: "${OBSERVABILITY_DATABASE_URL:?set OBSERVABILITY_DATABASE_URL to the observability PostgreSQL URL}"

command -v clickhouse-client >/dev/null || {
  echo "clickhouse-client is required (configure its connection/authentication before running)" >&2
  exit 1
}
command -v psql >/dev/null || {
  echo "psql is required" >&2
  exit 1
}

source_rows="$(mktemp)"
trap 'rm -f "$source_rows"' EXIT

# FINAL applies the ReplacingMergeTree collapse before filtering tombstones. Keep every alert name
# and product: zero-success thresholds share this table with success-rate-drop thresholds.
clickhouse-client --query "
SELECT
    name,
    product,
    trim(merchant_id) AS merchant_id,
    trim(ifNull(profile_id, '')) AS profile_id,
    min_volume,
    min_impacted_volume,
    tolerance,
    diff_threshold,
    updated_by,
    last_updated_at,
    0 AS is_deleted
FROM hyperswitch_alerts_sr_thresholds FINAL
WHERE is_deleted = 0
ORDER BY merchant_id, profile_id, name, product
FORMAT TabSeparated
" >"$source_rows"

source_count="$(wc -l <"$source_rows" | tr -d ' ')"

# Refuse to merge into a partially populated destination. This makes retries explicit and prevents
# a stale source snapshot from overwriting rows created after cutover.
destination_count="$(psql "$OBSERVABILITY_DATABASE_URL" -XAtqc \
  'SELECT count(*) FROM success_rate_threshold_overrides')"
if [[ "$destination_count" != "0" ]]; then
  echo "refusing backfill: destination contains $destination_count threshold rows" >&2
  exit 1
fi

{
  cat <<'SQL'
\set ON_ERROR_STOP on
BEGIN;
CREATE TEMP TABLE threshold_backfill
    (LIKE success_rate_threshold_overrides INCLUDING DEFAULTS);
COPY threshold_backfill (
    name, product, merchant_id, profile_id,
    min_volume, min_impacted_volume, tolerance, diff_threshold,
    updated_by, last_updated_at, is_deleted
) FROM STDIN;
SQL
  cat "$source_rows"
  cat <<'SQL'
\.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM threshold_backfill
        WHERE merchant_id = ''
           OR merchant_id <> btrim(merchant_id)
           OR profile_id <> btrim(profile_id)
           OR name = ''
           OR product = ''
           OR updated_by = ''
    ) THEN
        RAISE EXCEPTION 'source contains an invalid threshold key or actor';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM threshold_backfill
        GROUP BY name, product, merchant_id, profile_id
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'source keys collide after merchant/profile trimming';
    END IF;
END $$;

INSERT INTO success_rate_threshold_overrides (
    name, product, merchant_id, profile_id,
    min_volume, min_impacted_volume, tolerance, diff_threshold,
    updated_by, last_updated_at, is_deleted
)
SELECT
    name, product, merchant_id, profile_id,
    min_volume, min_impacted_volume, tolerance, diff_threshold,
    updated_by, last_updated_at, false
FROM threshold_backfill;
COMMIT;
SQL
} | psql "$OBSERVABILITY_DATABASE_URL" -X

destination_count="$(psql "$OBSERVABILITY_DATABASE_URL" -XAtqc \
  'SELECT count(*) FROM success_rate_threshold_overrides WHERE NOT is_deleted')"
if [[ "$destination_count" != "$source_count" ]]; then
  echo "backfill verification failed: source=$source_count destination=$destination_count" >&2
  exit 1
fi

echo "backfilled and verified $destination_count active threshold overrides"
