-- Your SQL goes here
ALTER TABLE dispute
ADD COLUMN evidence JSONB NOT NULL DEFAULT '{}'::JSONB;