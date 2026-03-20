-- Add skip_psp_tokenization column to payment_intent table
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS tokenization VARCHAR(64);