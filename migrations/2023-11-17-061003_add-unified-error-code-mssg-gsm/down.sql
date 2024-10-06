-- This file should undo anything in `up.sql`
ALTER TABLE gateway_status_map DROP COLUMN IF EXISTS unified_code;
ALTER TABLE gateway_status_map DROP COLUMN IF EXISTS unified_message;