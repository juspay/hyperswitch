-- Remove retry_type column from payment_attempt table
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS retry_type;
