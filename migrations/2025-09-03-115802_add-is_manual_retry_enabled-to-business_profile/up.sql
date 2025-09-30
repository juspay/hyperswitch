-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS is_manual_retry_enabled BOOLEAN;