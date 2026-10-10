CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_payment_methods_locker_fingerprint_id
    ON payment_methods (locker_fingerprint_id);
