-- This file should undo anything in `up.sql`
ALTER TABLE dispute DROP COLUMN IF EXISTS additional_details;
