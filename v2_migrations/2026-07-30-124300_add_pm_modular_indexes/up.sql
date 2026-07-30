CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_locker_fingerprint_id_index
ON payment_methods (locker_fingerprint_id)
WHERE locker_fingerprint_id IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_index
ON payment_methods (id)
WHERE id IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS customers_id_merchant_id_index
ON customers (id, merchant_id)
WHERE id IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS customers_merchant_id_reference_id_index
ON customers (merchant_id, merchant_reference_id)
WHERE merchant_reference_id IS NOT NULL;
