CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_payment_attempt_attempt_id_merchant_id
    ON payment_attempt (attempt_id, merchant_id);
