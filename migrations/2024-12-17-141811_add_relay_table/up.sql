-- Your SQL goes here
CREATE TYPE "RelayStatus" AS ENUM ('created', 'pending', 'failure', 'success');

CREATE TYPE "RelayType" AS ENUM ('refund');

CREATE TABLE relay (
    id VARCHAR(64) PRIMARY KEY,
    connector_resource_id VARCHAR(128) NOT NULL,
    connector_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    relay_type "RelayType" NOT NULL,
    request_data JSONB DEFAULT NULL,
    status "RelayStatus" NOT NULL,
    connector_reference_id VARCHAR(128),
    error_code VARCHAR(64),
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    response_data JSONB DEFAULT NULL
);

