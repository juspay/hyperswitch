-- Your SQL goes here
CREATE INDEX IF NOT EXISTS payment_attempt_payment_id_index ON payment_attempt (payment_id);

DROP INDEX IF EXISTS payment_attempt_payment_id_merchant_id_index;