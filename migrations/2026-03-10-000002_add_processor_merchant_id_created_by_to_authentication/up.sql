-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
