-- This file should undo anything in `up.sql`
ALTER TABLE invoice DROP COLUMN IF EXISTS connector_invoice_id;