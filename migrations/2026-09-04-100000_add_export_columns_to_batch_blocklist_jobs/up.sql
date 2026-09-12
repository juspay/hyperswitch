ALTER TABLE batch_blocklist_jobs
    ADD COLUMN IF NOT EXISTS job_type VARCHAR(32),
    ADD COLUMN IF NOT EXISTS file_name VARCHAR(255),
    ADD COLUMN IF NOT EXISTS file_key VARCHAR(512),
    ADD COLUMN IF NOT EXISTS error_message TEXT,
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMP;

-- Every row that predates exports is an upload, so the listing can filter on the value alone.
UPDATE batch_blocklist_jobs SET job_type = 'upload' WHERE job_type IS NULL;
