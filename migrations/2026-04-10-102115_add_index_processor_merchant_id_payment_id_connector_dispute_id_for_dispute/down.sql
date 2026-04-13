-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS idx_dispute_processor_merchant_id_payment_id_connector_dispute_id;
