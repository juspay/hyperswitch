-- Your SQL goes here
ALTER TABLE organization
ADD COLUMN IF NOT EXISTS version "ApiVersion" NOT NULL DEFAULT 'v1';