CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_attempt_id_processor_merchant_id_payment_attempt
    ON payment_attempt (attempt_id, processor_merchant_id);
