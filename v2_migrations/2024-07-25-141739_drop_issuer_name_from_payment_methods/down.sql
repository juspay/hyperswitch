-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS issuer_name VARCHAR(64);