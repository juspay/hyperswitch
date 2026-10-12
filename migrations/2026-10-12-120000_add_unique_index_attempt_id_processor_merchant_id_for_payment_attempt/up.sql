-- Your SQL goes here
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_attempt_id_processor_merchant_id_index ON payment_attempt (attempt_id, processor_merchant_id);
