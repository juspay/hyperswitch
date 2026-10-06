-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dispute_processor_merchant_id_dispute_id ON dispute (processor_merchant_id, dispute_id);
