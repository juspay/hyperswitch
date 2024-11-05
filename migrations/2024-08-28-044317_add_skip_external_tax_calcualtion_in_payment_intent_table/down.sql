-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN if EXISTS skip_external_tax_calculation;