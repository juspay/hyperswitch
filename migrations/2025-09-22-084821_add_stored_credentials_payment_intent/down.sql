-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN is_stored_credential;
ALTER TABLE payment_attempt DROP COLUMN is_stored_credential;
