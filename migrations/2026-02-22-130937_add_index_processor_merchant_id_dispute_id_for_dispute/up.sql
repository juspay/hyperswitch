-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS dispute_processor_merchant_id_dispute_id_index ON dispute (processor_merchant_id, dispute_id);
