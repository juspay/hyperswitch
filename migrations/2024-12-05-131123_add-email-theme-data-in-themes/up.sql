-- Your SQL goes here
ALTER TABLE themes ADD COLUMN IF NOT EXISTS email_primary_color VARCHAR(64) NOT NULL DEFAULT '#006DF9';
ALTER TABLE themes ADD COLUMN IF NOT EXISTS email_foreground_color VARCHAR(64) NOT NULL DEFAULT '#000000';
ALTER TABLE themes ADD COLUMN IF NOT EXISTS email_background_color VARCHAR(64) NOT NULL DEFAULT '#FFFFFF';
ALTER TABLE themes ADD COLUMN IF NOT EXISTS email_entity_name VARCHAR(64) NOT NULL DEFAULT 'Hyperswitch';
ALTER TABLE themes ADD COLUMN IF NOT EXISTS email_entity_logo_url TEXT NOT NULL DEFAULT 'https://app.hyperswitch.io/email-assets/HyperswitchLogo.png';
