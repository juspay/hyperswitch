ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS request_overcapture BOOLEAN;
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS overcapture_applied BOOLEAN;
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS maximum_capturable_amount BIGINT;