-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
DROP COLUMN is_iframe_redirection_enabled;