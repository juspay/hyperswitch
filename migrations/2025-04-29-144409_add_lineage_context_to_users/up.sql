-- Your SQL goes here
ALTER TABLE users ADD COLUMN IF NOT EXISTS lineage_context JSONB;
