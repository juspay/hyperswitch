-- Your SQL goes here
ALTER TABLE roles ADD COLUMN IF NOT EXISTS tenant_id VARCHAR(64) NOT NULL DEFAULT 'public';