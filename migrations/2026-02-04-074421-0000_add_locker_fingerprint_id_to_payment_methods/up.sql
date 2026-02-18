ALTER TABLE payment_methods
    ADD COLUMN IF NOT EXISTS locker_fingerprint_id VARCHAR(64);