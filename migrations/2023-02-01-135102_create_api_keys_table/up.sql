CREATE TABLE api_keys (
    key_id VARCHAR(64) NOT NULL PRIMARY KEY,
    merchant_id VARCHAR(64) NOT NULL,
    NAME VARCHAR(64) NOT NULL,
    description VARCHAR(256) DEFAULT NULL,
    hash_key VARCHAR(64) NOT NULL,
    hashed_api_key VARCHAR(128) NOT NULL,
    prefix VARCHAR(16) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    expires_at TIMESTAMP DEFAULT NULL,
    last_used TIMESTAMP DEFAULT NULL
);
