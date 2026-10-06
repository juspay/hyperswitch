ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS external_threeds_authentication_type VARCHAR(64);
