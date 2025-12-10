-- Your SQL goes here
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS standardised_code VARCHAR(64);
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS description VARCHAR(512);
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS user_guidance_message VARCHAR(512);
