ALTER TABLE payment_intent
    ADD COLUMN IF NOT EXISTS value_added_tax_details JSONB;
