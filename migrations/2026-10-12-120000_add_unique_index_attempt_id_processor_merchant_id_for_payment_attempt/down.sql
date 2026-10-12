-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS payment_attempt_attempt_id_processor_merchant_id_index;
