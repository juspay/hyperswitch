-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN separate_authentication,
DROP COLUMN authentication_provider,
DROP COLUMN authentication_id;