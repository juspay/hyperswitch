-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);

ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);

ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS default_profile VARCHAR(64);

-- Profile id is needed in refunds for listing refunds by business profile
ALTER TABLE refund
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);

-- For listing disputes by business profile
ALTER TABLE dispute
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);

-- For a similar use case as to payments
ALTER TABLE payout_attempt
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);
