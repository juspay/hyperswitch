-- Remove applied_offer_details from payment_attempt table
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS applied_offer_details;
