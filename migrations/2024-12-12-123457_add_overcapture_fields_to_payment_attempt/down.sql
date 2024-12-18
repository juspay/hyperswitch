ALTER TABLE payment_attempt DROP COLUMN IF EXISTS request_overcapture;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS overcapture_applied;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS maximum_capturable_amount;
