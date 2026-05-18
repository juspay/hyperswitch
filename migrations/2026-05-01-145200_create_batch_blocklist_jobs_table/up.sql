CREATE TABLE batch_blocklist_jobs (
    id VARCHAR(64) PRIMARY KEY,
    merchant_id VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    total_rows INTEGER NOT NULL,
    succeeded_rows INTEGER NOT NULL,
    failed_rows INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE INDEX batch_blocklist_jobs_merchant_id_created_at_idx
    ON batch_blocklist_jobs (merchant_id);
