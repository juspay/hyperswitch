-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS amount_capturable BIGINT NOT NULL DEFAULT 0;