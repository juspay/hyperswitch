
CREATE TYPE "RoleScope" AS ENUM ('merchant','organization');

CREATE TABLE IF NOT EXISTS roles (
    id SERIAL PRIMARY KEY,
    role_name VARCHAR(64) NOT NULL,
    role_id VARCHAR(64) NOT NULL UNIQUE,
    merchant_id VARCHAR(64) NOT NULL,
    org_id VARCHAR(64) NOT NULL,
    groups TEXT[] NOT NULL,
    scope "RoleScope" NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
	created_by VARCHAR(64) NOT NULL,
    last_modified_at TIMESTAMP NOT NULL DEFAULT now(),
	last_modified_by VARCHAR(64) NOT NULL
);

CREATE INDEX IF NOT EXISTS  role_id_index ON roles (role_id);

