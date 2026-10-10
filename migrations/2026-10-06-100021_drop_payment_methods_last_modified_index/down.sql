CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_last_modified_index
    ON payment_methods (last_modified);
