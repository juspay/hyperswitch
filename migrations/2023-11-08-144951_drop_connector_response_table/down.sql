-- This file should undo anything in `up.sql`
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

ALTER TABLE connector_response ALTER COLUMN connector_name DROP NOT NULL;
ALTER TABLE connector_response RENAME COLUMN txn_id TO attempt_id;
ALTER TABLE connector_response
    ALTER COLUMN payment_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN attempt_id TYPE VARCHAR(64),
    ALTER COLUMN connector_name TYPE VARCHAR(64),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(128);



ALTER TABLE connector_response
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE connector_response
ALTER COLUMN created_at DROP DEFAULT;

ALTER TABLE connector_response ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';
