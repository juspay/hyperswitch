-- Add skip_psp_tokenization column to payment_intent table
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS skip_psp_tokenization BOOLEAN;