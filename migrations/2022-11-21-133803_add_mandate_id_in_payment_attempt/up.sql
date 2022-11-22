-- Your SQL goes here
ALTER TABLE payment_attempt ADD IF NOT EXISTS mandate_id VARCHAR(255);
