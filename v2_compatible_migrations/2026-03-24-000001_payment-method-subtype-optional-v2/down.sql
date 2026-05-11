-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt ALTER COLUMN payment_method_subtype SET NOT NULL;
