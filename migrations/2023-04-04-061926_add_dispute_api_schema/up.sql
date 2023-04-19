-- Your SQL goes here
CREATE TYPE "FileUploadProvider" AS ENUM ('hyperswitch', 'stripe');

CREATE TABLE file_metadata (
    id SERIAL PRIMARY KEY,
    file_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    file_name VARCHAR(255),
    file_size INTEGER NOT NULL,
    file_type VARCHAR(255) NOT NULL,
    provider_file_id VARCHAR(255) NOT NULL,
    file_upload_provider "FileUploadProvider" NOT NULL,
    available BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);

CREATE UNIQUE INDEX merchant_id_file_id_index ON file_metadata (merchant_id, file_id);
