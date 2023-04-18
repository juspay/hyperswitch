ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS business_sub_label VARCHAR(64);
