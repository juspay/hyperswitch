-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS request_external_three_ds_authentication BOOLEAN;
