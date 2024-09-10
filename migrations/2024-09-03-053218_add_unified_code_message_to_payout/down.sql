ALTER TABLE payout_attempt
DROP COLUMN IF EXISTS unified_code,
DROP COLUMN IF EXISTS unified_message;