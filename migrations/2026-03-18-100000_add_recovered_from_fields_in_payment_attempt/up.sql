-- Add recovered_from fields to payment_attempt table for revenue recovery tracking
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS recovered_from_error_code VARCHAR(255) DEFAULT NULL,
ADD COLUMN IF NOT EXISTS recovered_from_error_reason TEXT DEFAULT NULL,
ADD COLUMN IF NOT EXISTS recovered_from_standardised_code VARCHAR(64) DEFAULT NULL,
ADD COLUMN IF NOT EXISTS recovered_from_connector VARCHAR(64) DEFAULT NULL;
