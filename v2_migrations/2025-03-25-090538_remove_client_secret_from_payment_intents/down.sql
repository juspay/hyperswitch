-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
ADD COLUMN client_secret VARCHAR(128);