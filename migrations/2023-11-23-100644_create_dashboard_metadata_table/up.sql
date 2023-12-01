-- Your SQL goes here

CREATE TABLE IF NOT EXISTS dashboard_metadata (
        id SERIAL PRIMARY KEY,
        user_id VARCHAR(64),
        merchant_id VARCHAR(64) NOT NULL,
        org_id VARCHAR(64) NOT NULL,
        data_key VARCHAR(64) NOT NULL,
        data_value JSON NOT NULL,
        created_by VARCHAR(64) NOT NULL,
        created_at TIMESTAMP NOT NULL DEFAULT now(),
        last_modified_by VARCHAR(64) NOT NULL,
        last_modified_at TIMESTAMP NOT NULL DEFAULT now()
    );

CREATE UNIQUE INDEX IF NOT EXISTS dashboard_metadata_index ON dashboard_metadata (
    COALESCE(user_id, '0'),
    merchant_id,
    org_id,
    data_key
);