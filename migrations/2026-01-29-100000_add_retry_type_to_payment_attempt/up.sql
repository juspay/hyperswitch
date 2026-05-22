-- Add retry_type column to payment_attempt table
ALTER TABLE payment_attempt ADD COLUMN retry_type VARCHAR(64);
