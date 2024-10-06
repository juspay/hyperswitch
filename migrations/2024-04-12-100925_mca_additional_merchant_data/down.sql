-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS additional_merchant_data;