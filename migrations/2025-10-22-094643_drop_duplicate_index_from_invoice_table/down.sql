-- This file should undo anything in `up.sql`
CREATE INDEX IF NOT EXISTS invoice_subscription_id_connector_invoice_id_index ON invoice (subscription_id, connector_invoice_id);