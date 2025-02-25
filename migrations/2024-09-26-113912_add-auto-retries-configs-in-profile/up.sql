-- Your SQL goes here
-- Add is_auto_retries_enabled column in business_profile table
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_auto_retries_enabled BOOLEAN;

-- Add max_auto_retries_enabled column in business_profile table
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS max_auto_retries_enabled SMALLINT;
