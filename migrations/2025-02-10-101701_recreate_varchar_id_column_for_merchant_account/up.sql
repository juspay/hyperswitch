-- Your SQL goes here
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS id VARCHAR(64);