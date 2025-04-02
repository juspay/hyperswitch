-- Your SQL goes here
ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS service_details JSONB
DEFAULT NULL;