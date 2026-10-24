-- Your SQL goes here
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_history TEXT[] DEFAULT NULL;
