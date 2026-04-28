-- Remove external_surcharge_details from payment_attempt table
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS external_surcharge_details;
