-- This file should undo anything in `up.sql`

ALTER TABLE blocklist DROP COLUMN IF EXISTS profile_id;

ALTER TABLE batch_blocklist_jobs DROP COLUMN IF EXISTS profile_id;
