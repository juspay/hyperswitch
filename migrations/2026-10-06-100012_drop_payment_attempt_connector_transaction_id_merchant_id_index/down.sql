CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_connector_transaction_id_merchant_id_index
    ON payment_attempt (connector_transaction_id, merchant_id);
