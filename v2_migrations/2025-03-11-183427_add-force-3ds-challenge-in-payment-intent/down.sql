-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
DROP COLUMN force_3ds_challenge;