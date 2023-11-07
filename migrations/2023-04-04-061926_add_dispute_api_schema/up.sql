-- Your SQL goes here
CREATE TABLE file_metadata (
    file_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    file_name VARCHAR(255),
    file_size INTEGER NOT NULL,
    file_type VARCHAR(255) NOT NULL,
    provider_file_id VARCHAR(255),
    file_upload_provider VARCHAR(255),
    available BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    PRIMARY KEY (file_id, merchant_id)
);
