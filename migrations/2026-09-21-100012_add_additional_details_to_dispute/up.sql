-- Your SQL goes here
ALTER TABLE dispute ADD COLUMN IF NOT EXISTS additional_details JSONB;
