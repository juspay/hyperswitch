-- Your SQL goes here
ALTER TABLE payout_attempt 
ADD COLUMN additional_payout_method_data JSONB DEFAULT NULL;