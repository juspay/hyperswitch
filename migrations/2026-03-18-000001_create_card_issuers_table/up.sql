-- Your SQL goes here
CREATE TABLE IF NOT EXISTS card_issuers (
    id VARCHAR(64) PRIMARY KEY,
    issuer_name VARCHAR NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_modified_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_card_issuers_name ON card_issuers (issuer_name);
