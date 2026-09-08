CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_locker_fingerprint_id_index
ON payment_methods (id, locker_fingerprint_id);
