CREATE INDEX CONCURRENTLY IF NOT EXISTS customers_merchant_id_merchant_reference_id_index
ON customers (merchant_id, merchant_reference_id)
WHERE merchant_reference_id IS NOT NULL;
