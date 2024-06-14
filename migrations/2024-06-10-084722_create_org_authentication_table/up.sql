-- Your SQL goes here
CREATE TYPE "AuthMethod" AS ENUM(
    'open_id_connect',
    'password',
    'magic_link'
);

CREATE TABLE IF NOT EXISTS org_authentication_methods (
    id SERIAL PRIMARY KEY,
    owner_id VARCHAR(64) NOT NULL,
    auth_method "AuthMethod" NOT NULL,
    config JSONB,
    allow_signup BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    last_modified_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT org_auth_method_unique UNIQUE (owner_id, auth_method)
);

CREATE INDEX IF NOT EXISTS org_id_auth_methods_index ON org_authentication_methods (owner_id);