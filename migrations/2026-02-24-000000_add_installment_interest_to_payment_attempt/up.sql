ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS installment_interest BIGINT;
