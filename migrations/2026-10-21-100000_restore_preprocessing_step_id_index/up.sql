-- UCS source webhooks resolve payment attempts by preprocessing_step_id.
CREATE INDEX CONCURRENTLY IF NOT EXISTS preprocessing_step_id_index
    ON payment_attempt (preprocessing_step_id);
