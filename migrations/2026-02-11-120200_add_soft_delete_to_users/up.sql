-- Your SQL goes here
-- Add is_active column to users table with default value TRUE
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
