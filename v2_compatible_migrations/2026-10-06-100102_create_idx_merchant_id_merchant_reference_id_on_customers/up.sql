-- Replaces customers_merchant_id_merchant_reference_id_index, which is partial
-- (WHERE merchant_reference_id IS NOT NULL); the target is unconditional.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_merchant_id_merchant_reference_id
    ON customers (merchant_id, merchant_reference_id);
