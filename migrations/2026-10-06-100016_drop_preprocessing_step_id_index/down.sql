CREATE INDEX CONCURRENTLY IF NOT EXISTS preprocessing_step_id_index
    ON payment_attempt (preprocessing_step_id);
