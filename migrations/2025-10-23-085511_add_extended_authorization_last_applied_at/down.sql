-- Remove the 'extended_authorization_last_applied_at' column from the 'payment_attempt' table
ALTER TABLE payment_attempt
DROP COLUMN extended_authorization_last_applied_at;