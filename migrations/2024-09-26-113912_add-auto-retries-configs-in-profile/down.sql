-- This file should undo anything in `up.sql`
-- Drop is_auto_retries_enabled column from business_profile table
ALTER TABLE business_profile DROP COLUMN IF EXISTS is_auto_retries_enabled;

-- Drop max_auto_retries_enabled column from business_profile table
ALTER TABLE business_profile DROP COLUMN IF EXISTS max_auto_retries_enabled;
