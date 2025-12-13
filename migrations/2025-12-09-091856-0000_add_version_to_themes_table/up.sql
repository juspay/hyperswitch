-- Your SQL goes here
ALTER TABLE themes
ADD COLUMN theme_config_version  VARCHAR(13) NOT NULL DEFAULT '0';