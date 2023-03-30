-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN active_attempt_id VARCHAR(64) NOT NULL DEFAULT 'xxx';

UPDATE payment_intent SET active_attempt_id = payment_attempt.attempt_id from payment_attempt where payment_intent.active_attempt_id = payment_attempt.payment_id;

CREATE UNIQUE INDEX payment_attempt_payment_id_merchant_id_attempt_id_index ON payment_attempt (payment_id, merchant_id, attempt_id);

-- Because payment_attempt table can have rows with same payment_id and merchant_id, this index is dropped.
DROP index payment_attempt_payment_id_merchant_id_index;

CREATE INDEX payment_attempt_payment_id_merchant_id_index ON payment_attempt (payment_id, merchant_id);
