-- Your SQL goes here
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_password_changed_at TIMESTAMP;