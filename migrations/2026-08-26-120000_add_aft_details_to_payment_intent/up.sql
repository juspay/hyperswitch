-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS is_account_funded_transaction BOOLEAN DEFAULT NULL;
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS recipient_details BYTEA DEFAULT NULL;
