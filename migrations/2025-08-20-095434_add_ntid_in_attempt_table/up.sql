ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS network_transaction_id VARCHAR(255) NULL;
