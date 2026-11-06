-- This file should undo anything in `up.sql`

ALTER TABLE payment_intent DROP COLUMN IF EXISTS value_added_tax_details;
