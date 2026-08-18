CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_active_index
ON payment_methods (id)
WHERE status = 'active';
