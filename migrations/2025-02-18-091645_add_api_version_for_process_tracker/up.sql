-- Your SQL goes here
ALTER TABLE
    process_tracker
ADD
    COLUMN IF NOT EXISTS version "ApiVersion" NOT NULL DEFAULT 'v1';