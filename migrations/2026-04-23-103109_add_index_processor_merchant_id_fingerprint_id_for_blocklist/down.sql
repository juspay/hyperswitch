-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS blocklist_processor_merchant_id_fingerprint_id_index;
