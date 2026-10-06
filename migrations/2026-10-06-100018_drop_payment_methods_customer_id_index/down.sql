CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_customer_id_index
    ON payment_methods (customer_id);
