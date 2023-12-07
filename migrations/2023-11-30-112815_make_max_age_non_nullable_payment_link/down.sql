-- This file should undo anything in `up.sql`
ALTER TABLE payment_link ALTER COLUMN expiry DROP NOT NULL;