-- Your SQL goes here
ALTER TABLE payment_attempt
ADD IF NOT EXISTS error_code VARCHAR(255) DEFAULT NULL;