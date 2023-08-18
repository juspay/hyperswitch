-- Your SQL goes here
ALTER TABLE payment_intent
ADD profile_id VARCHAR(64);

ALTER TABLE merchant_connector_account
ADD profile_id VARCHAR(64);

ALTER TABLE merchant_account
ADD default_profile VARCHAR(64);
