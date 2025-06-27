-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
DROP COLUMN IF EXISTS is_payment_id_from_merchant;