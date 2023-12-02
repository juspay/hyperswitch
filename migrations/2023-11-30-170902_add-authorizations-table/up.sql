-- Your SQL goes here

CREATE TYPE "AuthorizationStatus" AS ENUM (
    'success',
    'failure',
    'created',
    'unresolved'
);

CREATE TABLE IF NOT EXISTS "authorization" (
    authorization_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    payment_id VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    status "AuthorizationStatus" NOT NULL,
    code VARCHAR(255),
    message TEXT,
    connector_authorization_id VARCHAR(64),
    PRIMARY KEY (authorization_id, merchant_id)
);