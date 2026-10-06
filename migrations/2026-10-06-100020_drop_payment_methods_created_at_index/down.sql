CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_created_at_index
    ON payment_methods (created_at);
