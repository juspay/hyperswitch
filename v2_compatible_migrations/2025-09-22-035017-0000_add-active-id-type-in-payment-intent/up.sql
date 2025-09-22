ALTER TABLE payment_intent
    ADD COLUMN IF NOT EXISTS active_attempt_id_type VARCHAR(16);