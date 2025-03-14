-- This file should undo anything in `up.sql`
ALTER TABLE roles ALTER COLUMN entity_type DROP DEFAULT;

ALTER TABLE roles ALTER COLUMN entity_type DROP NOT NULL;