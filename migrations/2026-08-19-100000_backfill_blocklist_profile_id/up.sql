-- Create one blocklist entry per business profile for entries that predate profile scoping.
-- Safe to re-run: ON CONFLICT DO NOTHING skips entries that already exist. This backfill should be
-- executed again after deployment is complete, since older code keeps writing entries with a NULL
-- profile_id. The NULL entries are left in place so that a profile created later still inherits them.
INSERT INTO blocklist (merchant_id, fingerprint_id, data_kind, metadata,
                       created_at, processor_merchant_id, created_by, profile_id)
SELECT b.merchant_id, b.fingerprint_id, b.data_kind, b.metadata,
       b.created_at, COALESCE(b.processor_merchant_id, b.merchant_id), b.created_by, p.profile_id
  FROM blocklist b
  JOIN business_profile p
    ON p.merchant_id = COALESCE(b.processor_merchant_id, b.merchant_id)
 WHERE b.profile_id IS NULL
ON CONFLICT DO NOTHING;
