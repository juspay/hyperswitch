CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_index
ON payment_methods (id)
WHERE id IS NOT NULL;
