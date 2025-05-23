-- Your SQL goes here

-- Add authentication_client_secret column to the authentication table
ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS authentication_client_secret VARCHAR(128) NULL;
