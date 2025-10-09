-- Your SQL goes here
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS connector_invoice_id VARCHAR(64);
CREATE INDEX invoice_subscription_id_connector_invoice_id_index ON invoice (subscription_id, connector_invoice_id);