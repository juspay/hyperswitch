-- Your SQL goes here
ALTER TABLE PAYMENT_INTENT ADD COLUMN attempt_id VARCHAR(255) DEFAULT NULL;
CREATE UNIQUE INDEX payment_attempt_attempt_id_index ON payment_attempt (attempt_id);
DROP index payment_attempt_payment_id_merchant_id_index;