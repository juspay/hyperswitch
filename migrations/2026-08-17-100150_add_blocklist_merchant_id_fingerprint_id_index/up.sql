CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS blocklist_merchant_id_fingerprint_id_profile_id_index ON blocklist (merchant_id, fingerprint_id, profile_id);
