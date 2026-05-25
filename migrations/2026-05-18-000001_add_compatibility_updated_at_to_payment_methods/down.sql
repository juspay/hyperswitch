ALTER TABLE payment_methods
    DROP COLUMN IF EXISTS auxiliary_fingerprint_id,
    DROP COLUMN IF EXISTS compatibility_updated_at;
