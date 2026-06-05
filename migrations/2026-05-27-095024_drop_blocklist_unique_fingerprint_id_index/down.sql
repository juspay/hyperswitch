-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS blocklist_unique_fingerprint_id_index ON blocklist (merchant_id, fingerprint_id);
