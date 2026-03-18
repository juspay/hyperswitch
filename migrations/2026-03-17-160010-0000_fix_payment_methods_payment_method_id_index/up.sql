-- Step 1: Create unique index without blocking writes
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_payment_methods_payment_method_id_unique
ON payment_methods (payment_method_id)
WHERE payment_method_id IS NOT NULL;

-- Step 2: Drop old index safely
DROP INDEX CONCURRENTLY IF EXISTS idx_payment_methods_payment_method_id;