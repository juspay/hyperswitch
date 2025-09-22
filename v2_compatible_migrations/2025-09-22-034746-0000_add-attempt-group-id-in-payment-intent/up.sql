ALTER TABLE payment_intent
    ADD COLUMN IF NOT EXISTS active_attempts_group_id VARCHAR(64);