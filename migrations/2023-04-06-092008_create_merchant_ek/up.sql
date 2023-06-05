CREATE TABLE merchant_key_store(
    merchant_id VARCHAR(255) NOT NULL PRIMARY KEY,
    key bytea NOT NULL,
    created_at TIMESTAMP NOT NULL
);

