-- Target uses a hash index on payment_methods.customer_id; v1 created a btree
-- named payment_methods_customer_id_index, dropped in a later migration.
CREATE INDEX CONCURRENTLY IF NOT EXISTS customer_id_index
    ON payment_methods USING hash (customer_id);
