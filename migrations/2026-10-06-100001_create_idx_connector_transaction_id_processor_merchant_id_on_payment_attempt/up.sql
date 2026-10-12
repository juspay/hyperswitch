-- Name is the 63-character truncation of
-- idx_connector_transaction_id_processor_merchant_id_payment_attempt, spelled out
-- so the stored identifier matches exactly and Postgres emits no truncation notice.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_connector_transaction_id_processor_merchant_id_payment_atte
    ON payment_attempt (connector_transaction_id, processor_merchant_id);
