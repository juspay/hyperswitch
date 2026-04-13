-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dispute_processor_merchant_id_payment_id_connector_dispute_id ON dispute (processor_merchant_id, payment_id, connector_dispute_id);
