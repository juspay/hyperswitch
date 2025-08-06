ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS network_advice_code,
DROP COLUMN IF EXISTS networ_decline_code,
DROP COLUMN IF EXISTS network_error_message;