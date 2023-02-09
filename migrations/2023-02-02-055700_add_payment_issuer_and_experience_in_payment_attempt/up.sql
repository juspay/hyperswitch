-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS payment_issuer VARCHAR(50);

ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS payment_experience VARCHAR(50);
