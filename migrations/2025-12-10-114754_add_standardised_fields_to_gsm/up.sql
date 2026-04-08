-- Your SQL goes here
ALTER TABLE gateway_status_map
    ADD COLUMN IF NOT EXISTS standardised_code VARCHAR(64),
    ADD COLUMN IF NOT EXISTS description VARCHAR(1024),
    ADD COLUMN IF NOT EXISTS user_guidance_message VARCHAR(1024);
