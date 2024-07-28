-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);
