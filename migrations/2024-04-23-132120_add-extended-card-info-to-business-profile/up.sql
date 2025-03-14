-- Your SQL goes here

ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_extended_card_info_enabled BOOLEAN DEFAULT FALSE;