-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS refund_processor_merchant_id_refund_id_index;
