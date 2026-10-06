CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_payment_id_merchant_id_index
    ON payment_attempt (payment_id, merchant_id);
