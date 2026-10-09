CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_locker_id_index
ON payment_methods (locker_id)
WHERE locker_id IS NOT NULL;
