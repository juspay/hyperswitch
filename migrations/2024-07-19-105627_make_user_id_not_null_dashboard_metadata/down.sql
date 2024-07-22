-- This file should undo anything in `up.sql`
ALTER TABLE dashboard_metadata ALTER COLUMN user_id DROP NOT NULL;