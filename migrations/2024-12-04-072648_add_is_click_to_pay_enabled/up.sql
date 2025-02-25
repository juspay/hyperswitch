-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_click_to_pay_enabled BOOLEAN NOT NULL DEFAULT FALSE;