-- Your SQL goes here

ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS extended_card_info_config JSONB DEFAULT NULL;