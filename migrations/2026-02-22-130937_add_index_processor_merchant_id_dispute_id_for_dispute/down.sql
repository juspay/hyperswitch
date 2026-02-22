-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS dispute_processor_merchant_id_dispute_id_index;
