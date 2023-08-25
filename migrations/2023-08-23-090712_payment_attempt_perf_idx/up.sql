-- Your SQL goes here
CREATE INDEX payment_attempt_attempt_id_merchant_id_index ON payment_attempt (attempt_id, merchant_id);

