-- Your SQL goes here
CREATE TABLE connector_response (
    id SERIAL PRIMARY KEY,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    txn_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    connector_name VARCHAR(32) NOT NULL,
    connector_transaction_id VARCHAR(255),
    authentication_data JSON,
    encoded_data TEXT
);

CREATE UNIQUE INDEX connector_response_id_index ON connector_response (payment_id, merchant_id, txn_id);