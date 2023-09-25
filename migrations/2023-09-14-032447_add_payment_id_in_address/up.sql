-- Your SQL goes here
ALTER TABLE address ADD COLUMN payment_id VARCHAR(64);
ALTER TABLE customers ADD COLUMN address_id VARCHAR(64);