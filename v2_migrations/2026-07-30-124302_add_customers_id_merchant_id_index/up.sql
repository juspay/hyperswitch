CREATE INDEX CONCURRENTLY IF NOT EXISTS customers_id_merchant_id_index
ON customers (id, merchant_id)
WHERE id IS NOT NULL;
