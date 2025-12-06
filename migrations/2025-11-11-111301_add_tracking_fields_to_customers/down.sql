-- This file should undo anything in `up.sql`
-- Remove created_by column from customers table
ALTER TABLE customers DROP COLUMN IF EXISTS created_by;

-- Remove last_modified_by column from customers table
ALTER TABLE customers DROP COLUMN IF EXISTS last_modified_by;
