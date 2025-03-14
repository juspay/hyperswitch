-- Your SQL goes here
ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS version "ApiVersion" NOT NULL DEFAULT 'v1';