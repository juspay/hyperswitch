ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS merchant_fingerprint_id VARCHAR(64);