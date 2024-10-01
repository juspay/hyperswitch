-- This file should undo anything in `up.sql`
-- Make active_attempt_id mandatory in payment_intent
ALTER TABLE payment_intent ALTER COLUMN active_attempt_id SET NOT NULL;

-- Make session_expiry optional in payment_intent
ALTER TABLE payment_intent ALTER COLUMN session_expiry DROP NOT NULL;
