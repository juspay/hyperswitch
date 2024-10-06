-- Your SQL goes here
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS unified_code VARCHAR(255);
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS unified_message VARCHAR(1024);