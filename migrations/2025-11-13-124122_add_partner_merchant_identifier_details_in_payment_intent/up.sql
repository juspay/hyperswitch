-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS partner_merchant_identifier_details jsonb;