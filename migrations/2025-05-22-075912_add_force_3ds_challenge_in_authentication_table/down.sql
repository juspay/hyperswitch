-- This file should undo anything in `up.sql`

ALTER TABLE authentication
DROP COLUMN IF EXISTS force_3ds_challenge;
