-- Your SQL goes here
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_password_modified_at TIMESTAMP;