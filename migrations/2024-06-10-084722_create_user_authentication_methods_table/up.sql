-- Your SQL goes here
CREATE TABLE IF NOT EXISTS user_authentication_methods (
    id VARCHAR(64) PRIMARY KEY,
    auth_id VARCHAR(64) NOT NULL,
    owner_id VARCHAR(64) NOT NULL,
    owner_type VARCHAR(64) NOT NULL,
    auth_type VARCHAR(64) NOT NULL,
    private_config bytea,
    public_config JSONB,
    allow_signup BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    last_modified_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS auth_id_index ON user_authentication_methods (auth_id);
CREATE INDEX IF NOT EXISTS owner_id_index ON user_authentication_methods (owner_id);
