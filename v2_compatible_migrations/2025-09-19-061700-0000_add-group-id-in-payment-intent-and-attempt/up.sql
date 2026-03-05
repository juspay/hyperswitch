ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS attempts_group_id VARCHAR(64);

ALTER TABLE payment_intent
    ADD COLUMN IF NOT EXISTS active_attempts_group_id VARCHAR(64);

ALTER TABLE payment_intent
    ADD COLUMN IF NOT EXISTS active_attempt_id_type VARCHAR(16);