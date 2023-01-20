-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN attempt_id VARCHAR(255) NOT NULL DEFAULT 'deprecated_payment_intent';
ALTER TABLE payment_intent ALTER COLUMN attempt_id DROP DEFAULT;
CREATE UNIQUE INDEX payment_attempt_attempt_id_index ON payment_attempt (attempt_id);
DROP index payment_attempt_payment_id_merchant_id_index;