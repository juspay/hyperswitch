-- Your SQL goes here
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS connector_invoice_id VARCHAR(255);