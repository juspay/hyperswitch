-- This file should undo anything in `up.sql`
ALTER TABLE invoice DROP CONSTRAINT IF EXISTS invoice_subscription_id_connector_invoice_id_unique_index;
DROP INDEX IF EXISTS invoice_subscription_id_connector_invoice_id_unique_index;