-- Your SQL goes here

CREATE TABLE IF NOT EXISTS incremental_authorization (
    authorization_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    payment_id VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    status VARCHAR(64) NOT NULL,
    error_code VARCHAR(255),
    error_message TEXT,
    connector_authorization_id VARCHAR(64),
    previously_authorized_amount BIGINT NOT NULL,
    PRIMARY KEY (authorization_id, merchant_id)
);