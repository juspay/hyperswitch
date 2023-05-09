-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN preprocessing_step_id VARCHAR DEFAULT NULL;
CREATE INDEX preprocessing_step_id_index ON payment_attempt (preprocessing_step_id);
