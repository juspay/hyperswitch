-- Your SQL goes here
UPDATE roles SET entity_type = 'merchant' WHERE entity_type IS NULL;

ALTER TABLE roles ALTER COLUMN entity_type SET DEFAULT 'merchant';

ALTER TABLE roles ALTER COLUMN entity_type SET NOT NULL;