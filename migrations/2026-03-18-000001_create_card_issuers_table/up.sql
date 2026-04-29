-- Your SQL goes here
CREATE TABLE IF NOT EXISTS card_issuers (
    id VARCHAR(64) PRIMARY KEY,
    issuer_name VARCHAR NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);
