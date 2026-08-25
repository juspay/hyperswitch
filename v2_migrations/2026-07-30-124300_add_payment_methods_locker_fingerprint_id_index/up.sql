CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_locker_fingerprint_id_index
ON payment_methods (locker_fingerprint_id)
WHERE locker_fingerprint_id IS NOT NULL;
