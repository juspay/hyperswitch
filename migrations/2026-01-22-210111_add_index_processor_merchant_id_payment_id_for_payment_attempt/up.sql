-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_processor_merchant_id_payment_id_index ON payment_attempt (processor_merchant_id, payment_id);