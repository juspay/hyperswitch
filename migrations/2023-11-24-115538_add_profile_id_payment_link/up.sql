-- Your SQL goes here
ALTER TABLE payment_link ADD IF NOT EXISTS COLUMN profile_id VARCHAR(64);