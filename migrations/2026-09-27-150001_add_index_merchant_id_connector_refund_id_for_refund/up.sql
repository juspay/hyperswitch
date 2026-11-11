-- Your SQL goes here
-- Supports the stagger-release fallback lookup by merchant_id (for refunds with NULL processor_merchant_id).
-- TODO: Drop this index once processor_merchant_id is backfilled and the merchant_id fallback is removed.
CREATE INDEX CONCURRENTLY IF NOT EXISTS refund_merchant_id_connector_refund_id_index ON refund (merchant_id, connector_refund_id);
