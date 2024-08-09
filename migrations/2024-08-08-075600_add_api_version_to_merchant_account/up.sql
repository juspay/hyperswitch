-- Your SQL goes here
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS version "ApiVersion" NOT NULL DEFAULT 'v1';
