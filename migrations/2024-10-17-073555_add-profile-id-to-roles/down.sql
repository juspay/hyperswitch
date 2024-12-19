-- This file should undo anything in `up.sql`
ALTER TABLE roles DROP COLUMN IF EXISTS profile_id;