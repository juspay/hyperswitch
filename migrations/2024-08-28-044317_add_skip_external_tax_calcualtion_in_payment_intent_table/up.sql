-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS skip_external_tax_calculation BOOLEAN;