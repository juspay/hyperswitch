CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_customers_merchant_id_customer_id
    ON customers (merchant_id, customer_id);
