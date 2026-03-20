-- Your SQL goes here
ALTER TABLE payment_attempt
-- stores the date and time at which extended authorization was last applied on this payment
ADD COLUMN extended_authorization_last_applied_at timestamp;