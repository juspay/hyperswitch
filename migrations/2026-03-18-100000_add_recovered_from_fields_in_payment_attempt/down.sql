-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS recovered_from_error_code,
DROP COLUMN IF EXISTS recovered_from_error_reason,
DROP COLUMN IF EXISTS recovered_from_standardised_code,
DROP COLUMN IF EXISTS recovered_from_connector;
