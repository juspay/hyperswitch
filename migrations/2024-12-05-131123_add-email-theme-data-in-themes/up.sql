-- Your SQL goes here
ALTER TABLE themes ADD COLUMN email_primary_color VARCHAR(64) NOT NULL DEFAULT '#000000';
ALTER TABLE themes ADD COLUMN email_secondary_color VARCHAR(64) NOT NULL DEFAULT '#000000';
ALTER TABLE themes ADD COLUMN email_entity_name VARCHAR(64) NOT NULL DEFAULT 'Hyperswitch';
ALTER TABLE themes ADD COLUMN email_entity_logo VARCHAR(255) NOT NULL DEFAULT 'https://app.hyperswitch.io/email-assets/HyperswitchLogo.png';
