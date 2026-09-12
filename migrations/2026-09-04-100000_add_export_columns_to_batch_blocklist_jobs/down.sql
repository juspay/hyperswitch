ALTER TABLE batch_blocklist_jobs
    DROP COLUMN IF EXISTS expires_at,
    DROP COLUMN IF EXISTS error_message,
    DROP COLUMN IF EXISTS file_key,
    DROP COLUMN IF EXISTS file_name,
    DROP COLUMN IF EXISTS job_type;
