-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS network_token_requestor_reference_id VARCHAR(128) DEFAULT NULL;

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS network_token_locker_id VARCHAR(64) DEFAULT NULL;

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS network_token_payment_method_data BYTEA DEFAULT NULL;