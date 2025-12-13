-- This file should undo anything in `up.sql`
ALTER TABLE themes
DROP COLUMN  IF EXISTS theme_config_version;