-- This file should undo anything in `up.sql`
ALTER TABLE gateway_status_map DROP COLUMN error_category;

ALTER TABLE gateway_status_map DROP COLUMN error_sub_category;
