ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS attempts_group_id VARCHAR(64);