ALTER TABLE payment_methods
    ADD COLUMN IF NOT EXISTS auxiliary_fingerprint_id VARCHAR(64);
