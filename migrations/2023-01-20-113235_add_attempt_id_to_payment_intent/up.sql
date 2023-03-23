-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN attempt_id VARCHAR(64) NOT NULL DEFAULT 'xxx';

UPDATE payment_intent SET attempt_id = payment_attempt.attempt_id from payment_attempt where payment_intent.payment_id = payment_attempt.payment_id;

CREATE UNIQUE INDEX payment_attempt_attempt_id_merchant_id_index ON payment_attempt (attempt_id, merchant_id);

-- Because payment_attempt table can have rows with same payment_id and merchant_id, this index is dropped.
DROP index payment_attempt_payment_id_merchant_id_index;

CREATE INDEX payment_attempt_payment_id_merchant_id_index ON payment_attempt (payment_id, merchant_id);