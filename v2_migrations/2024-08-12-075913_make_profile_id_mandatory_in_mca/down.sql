-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account ALTER COLUMN profile_id DROP NOT NULL;