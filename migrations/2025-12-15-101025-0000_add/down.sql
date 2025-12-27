-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP COLUMN IF EXISTS customer_document_number;