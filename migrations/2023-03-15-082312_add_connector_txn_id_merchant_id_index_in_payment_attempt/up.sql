-- Your SQL goes here
CREATE UNIQUE INDEX payment_attempt_connector_transaction_id_merchant_id_index ON payment_attempt (connector_transaction_id, merchant_id);
