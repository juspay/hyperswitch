-- Your SQL goes here
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS blocklist_processor_merchant_id_fingerprint_id_index ON blocklist (processor_merchant_id, fingerprint_id);
