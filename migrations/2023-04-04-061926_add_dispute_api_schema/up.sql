-- Your SQL goes here
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    file_id VARCHAR(255) NOT NULL,
    file_size VARCHAR(255) NOT NULL,
    file_type VARCHAR(255) NOT NULL,
    provider_file_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);
