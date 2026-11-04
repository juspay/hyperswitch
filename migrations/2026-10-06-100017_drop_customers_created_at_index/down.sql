CREATE INDEX CONCURRENTLY IF NOT EXISTS customers_created_at_index
    ON customers (created_at);
