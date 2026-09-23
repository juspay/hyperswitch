ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS fingerprint_id VARCHAR(64);