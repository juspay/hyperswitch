-- Built concurrently here and promoted to the primary key in the next migration,
-- so the ACCESS EXCLUSIVE lock covers only the catalog change, not an index build.
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS customers_pkey
    ON customers (customer_id, merchant_id);
