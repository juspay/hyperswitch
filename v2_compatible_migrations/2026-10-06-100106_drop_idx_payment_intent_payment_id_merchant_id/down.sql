CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_payment_intent_payment_id_merchant_id
    ON payment_intent (payment_id, merchant_id);
