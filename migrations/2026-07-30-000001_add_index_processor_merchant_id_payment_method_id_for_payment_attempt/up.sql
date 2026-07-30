-- Your SQL goes here
-- Use CONCURRENTLY so building this index does not block writes on payment_attempt.
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_processor_merchant_id_payment_method_id_index ON payment_attempt (processor_merchant_id, payment_method_id);
