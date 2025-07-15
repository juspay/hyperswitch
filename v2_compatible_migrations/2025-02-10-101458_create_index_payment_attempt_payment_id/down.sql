-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS payment_attempt_payment_id_index;
CREATE INDEX IF NOT EXISTS payment_attempt_payment_id_merchant_id_index ON payment_attempt (payment_id, merchant_id);