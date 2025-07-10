-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS amount bigint;
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS currency "Currency";

