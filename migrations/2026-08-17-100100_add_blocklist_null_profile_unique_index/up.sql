CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS blocklist_pm_fingerprint_null_profile_index ON blocklist (processor_merchant_id, fingerprint_id) WHERE profile_id IS NULL;
