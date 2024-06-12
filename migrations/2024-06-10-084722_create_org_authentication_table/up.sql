-- Your SQL goes here
CREATE TYPE "AuthMethod" AS ENUM (
    'okta',
    'password',
    'magic_link'
);

CREATE TABLE IF NOT EXISTS org_authentication_methods (
    id SERIAL PRIMARY KEY,
	org_id VARCHAR(64) NOT NULL, 
    auth_method "AuthMethod" NOT NULL,
    auth_config JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    last_modified_at TIMESTAMP NOT NULL DEFAULT now(),
	CONSTRAINT org_auth_method_unique UNIQUE (org_id, auth_method)
);

CREATE INDEX IF NOT EXISTS org_id_auth_methods_index ON org_authentication_methods(org_id);