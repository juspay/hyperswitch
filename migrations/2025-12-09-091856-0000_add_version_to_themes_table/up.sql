-- Your SQL goes here
ALTER TABLE themes
ADD COLUMN IF NOT EXISTS theme_config_version VARCHAR(32) NOT NULL DEFAULT extract(epoch from now())::text;