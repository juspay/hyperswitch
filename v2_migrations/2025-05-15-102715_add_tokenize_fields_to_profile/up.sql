-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS tokenize_fields TEXT[] DEFAULT NULL; 