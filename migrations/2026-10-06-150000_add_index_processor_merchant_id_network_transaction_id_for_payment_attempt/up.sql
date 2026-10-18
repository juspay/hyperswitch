-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_processor_merchant_id_network_transaction_id_index ON payment_attempt (processor_merchant_id, network_transaction_id);
