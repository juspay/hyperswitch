ALTER TABLE payment_attempt
    DROP COLUMN IF EXISTS attempts_group_id;

ALTER TABLE payment_intent
    DROP COLUMN IF EXISTS active_attempts_group_id;

ALTER TABLE payment_intent
    DROP COLUMN IF EXISTS active_attempt_id_type;