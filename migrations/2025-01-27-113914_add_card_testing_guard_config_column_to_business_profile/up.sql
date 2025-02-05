-- Your SQL goes here

ALTER TABLE business_profile
ADD COLUMN card_testing_guard_config JSONB
DEFAULT NULL;

ALTER TABLE business_profile 
ADD COLUMN card_testing_secret_key BYTEA
DEFAULT NULL;