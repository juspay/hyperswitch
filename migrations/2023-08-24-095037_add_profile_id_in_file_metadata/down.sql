-- This file should undo anything in `up.sql`
ALTER TABLE file_metadata DROP COLUMN IF EXISTS profile_id;
