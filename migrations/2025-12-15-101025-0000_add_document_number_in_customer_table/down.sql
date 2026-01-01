-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP COLUMN IF EXISTS document_number;