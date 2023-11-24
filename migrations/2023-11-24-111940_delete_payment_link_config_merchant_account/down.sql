-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS payment_link_config JSONB NULL;