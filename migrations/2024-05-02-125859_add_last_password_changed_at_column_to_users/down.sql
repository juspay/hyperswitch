-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN IF EXISTS last_password_changed_at;