-- Add applied_offer_details to payment_attempt table
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS applied_offer_details JSONB;
