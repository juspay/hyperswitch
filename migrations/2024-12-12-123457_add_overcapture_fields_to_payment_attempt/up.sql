ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS overcapture_details JSONB DEFAULT NULL;