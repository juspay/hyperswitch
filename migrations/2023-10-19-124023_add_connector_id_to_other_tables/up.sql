-- Your SQL goes here
ALTER TABLE file_metadata
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);

ALTER TABLE refund
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);

ALTER TABLE payout_attempt
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);

ALTER TABLE dispute
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);

ALTER TABLE mandate
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);
