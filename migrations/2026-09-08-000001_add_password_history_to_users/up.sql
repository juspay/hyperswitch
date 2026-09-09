-- Your SQL goes here
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_history TEXT[] DEFAULT NULL;

-- Backfill so every existing user is protected against reusing their current password
-- from day one. Older passwords were never stored, so history fills in over time.
UPDATE users SET password_history = ARRAY[password] WHERE password IS NOT NULL;
