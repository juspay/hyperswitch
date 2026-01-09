-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN is_manual_retry_enabled;