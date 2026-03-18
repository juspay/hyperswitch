-- Step 1: Create the non-unique index without blocking writes
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_payment_methods_payment_method_id
ON payment_methods (payment_method_id);

-- Step 2: Drop the unique index safely
DROP INDEX CONCURRENTLY IF EXISTS idx_payment_methods_payment_method_id_unique;