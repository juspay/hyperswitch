-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS refund_processor_merchant_id_refund_id_index ON refund (processor_merchant_id, refund_id);
