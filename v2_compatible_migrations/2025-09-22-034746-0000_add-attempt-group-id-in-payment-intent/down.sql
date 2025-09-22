ALTER TABLE payment_intent
    DROP COLUMN IF EXISTS active_attempts_group_id;