-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS last_used_at;