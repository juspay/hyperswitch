-- Your SQL goes here
ALTER TABLE payment_link ADD COLUMN IF NOT EXISTS payment_link_config JSONB NULL;
