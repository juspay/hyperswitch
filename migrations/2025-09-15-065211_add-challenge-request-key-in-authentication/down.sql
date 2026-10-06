-- This file should undo anything in `up.sql`
ALTER TABLE authentication DROP COLUMN IF EXISTS challenge_request_key;