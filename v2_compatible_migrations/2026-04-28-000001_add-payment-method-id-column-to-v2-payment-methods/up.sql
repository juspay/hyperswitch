ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS payment_method_id VARCHAR(64);
