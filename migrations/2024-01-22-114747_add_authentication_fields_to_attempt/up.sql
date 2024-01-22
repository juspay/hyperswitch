-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN separate_authentication BOOLEAN default false,
ADD COLUMN authentication_provider VARCHAR(64),
ADD COLUMN authentication_id VARCHAR(64);