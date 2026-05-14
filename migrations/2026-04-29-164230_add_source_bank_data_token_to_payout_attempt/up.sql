ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS source_bank_data_token VARCHAR(64);
ALTER TABLE payout_attempt 
ADD COLUMN IF NOT EXISTS additional_source_bank_data JSONB DEFAULT NULL;