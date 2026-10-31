-- Your SQL goes here

ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS value_added_tax_amount BIGINT,
ADD COLUMN IF NOT EXISTS value_added_tax_percent DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS vat_invoice_reference_number VARCHAR;
