-- This file should undo anything in `up.sql`
-- Remove created_by column from payment_methods table
ALTER TABLE payment_methods DROP COLUMN IF EXISTS created_by;

-- Remove last_modified_by column from payment_methods table
ALTER TABLE payment_methods DROP COLUMN IF EXISTS last_modified_by;
