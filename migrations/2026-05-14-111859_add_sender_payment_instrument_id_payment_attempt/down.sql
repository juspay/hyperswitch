-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN sender_payment_instrument_id;