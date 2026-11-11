ALTER TABLE batch_blocklist_jobs
ADD COLUMN IF NOT EXISTS metadata JSONB;
