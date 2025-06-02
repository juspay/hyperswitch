-- Your SQL goes here

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS force_3ds_challenge BOOLEAN NULL;
