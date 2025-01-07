ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS request_overcapture,
DROP COLUMN IF EXISTS overcapture_status;
