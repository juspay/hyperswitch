-- This file should undo anything in `up.sql`
ALTER TABLE events DROP COLUMN IF EXISTS is_overall_delivery_successful;