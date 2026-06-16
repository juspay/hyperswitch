-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';
