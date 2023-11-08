-- Your SQL goes here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(64) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
	password VARCHAR(255) NOT NULL,
    is_verified bool NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    last_modified_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX user_id_index ON users (user_id);
CREATE UNIQUE INDEX user_email_index ON users (email);