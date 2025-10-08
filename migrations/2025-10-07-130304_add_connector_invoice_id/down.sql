-- This file should undo anything in `up.sql`
ALTER TABLE invoice DROP COLUMN IF EXISTS connector_invoice_id;
DROP INDEX IF EXISTS invoice_subscription_id_connector_invoice_id_index;