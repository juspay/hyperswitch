-- Your SQL goes here
ALTER TABLE payout_attempt 
ADD COLUMN IF NOT EXISTS additional_payout_method_data JSONB DEFAULT NULL;