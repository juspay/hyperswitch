CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_id_index
    ON payment_attempt (id);
