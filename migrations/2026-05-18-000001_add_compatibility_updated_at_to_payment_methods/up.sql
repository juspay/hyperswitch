ALTER TABLE payment_methods
    ADD COLUMN IF NOT EXISTS compatibility_updated_at TIMESTAMP;
