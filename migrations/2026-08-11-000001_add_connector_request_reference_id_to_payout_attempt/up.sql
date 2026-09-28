ALTER TABLE payout_attempt
ADD COLUMN IF NOT EXISTS connector_request_reference_id VARCHAR(255);
