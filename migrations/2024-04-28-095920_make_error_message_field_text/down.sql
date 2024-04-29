-- This file should undo anything in `up.sql`
ALTER TABLE authentication ALTER COLUMN error_message VARCHAR(64);