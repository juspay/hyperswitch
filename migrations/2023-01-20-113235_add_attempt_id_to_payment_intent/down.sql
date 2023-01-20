-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX payment_attempt_payment_id_merchant_id_index ON payment_attempt (payment_id, merchant_id);
DROP INDEX payment_attempt_attempt_id_index;
ALTER TABLE PAYMENT_INTENT DROP COLUMN attempt_id;