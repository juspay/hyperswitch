-- Your SQL goes here
CREATE TABLE IF NOT EXISTS user_roles (
	id SERIAL PRIMARY KEY,
	user_id VARCHAR(64) NOT NULL,
	merchant_id VARCHAR(64) NOT NULL,
	role_id VARCHAR(64) NOT NULL,
	org_id VARCHAR(64) NOT NULL, 
	status VARCHAR(64) NOT NULL, 
	created_by VARCHAR(64) NOT NULL,
	last_modified_by VARCHAR(64) NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT now(),
	last_modified_at TIMESTAMP NOT NULL DEFAULT now(),
	CONSTRAINT user_merchant_unique UNIQUE (user_id, merchant_id)
);


CREATE INDEX IF NOT EXISTS  user_id_roles_index ON user_roles (user_id);
CREATE INDEX IF NOT EXISTS  user_mid_roles_index ON user_roles (merchant_id);