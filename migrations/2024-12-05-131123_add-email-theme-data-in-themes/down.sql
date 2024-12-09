-- This file should undo anything in `up.sql`
ALTER TABLE themes DROP COLUMN email_primary_color;
ALTER TABLE themes DROP COLUMN email_foreground_color;
ALTER TABLE themes DROP COLUMN email_background_color;
ALTER TABLE themes DROP COLUMN email_entity_name;
ALTER TABLE themes DROP COLUMN email_entity_logo;
