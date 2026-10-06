CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_intent_id_index
    ON payment_intent (id);
