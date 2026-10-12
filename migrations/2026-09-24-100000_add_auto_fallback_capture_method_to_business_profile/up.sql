-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS auto_fallback_capture_method VARCHAR(16);
