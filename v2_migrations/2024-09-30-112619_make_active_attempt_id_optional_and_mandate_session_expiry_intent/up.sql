-- Your SQL goes here
-- Make active_attempt_id optional in payment_intent
ALTER TABLE payment_intent ALTER COLUMN active_attempt_id DROP NOT NULL;

-- Make session_expiry mandatory in payment_intent
ALTER TABLE payment_intent ALTER COLUMN session_expiry SET NOT NULL;
