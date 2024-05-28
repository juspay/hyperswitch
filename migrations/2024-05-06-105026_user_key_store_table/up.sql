-- Your SQL goes here
CREATE TABLE IF NOT EXISTS user_key_store (
    user_id VARCHAR(64) PRIMARY KEY,
    key bytea NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
