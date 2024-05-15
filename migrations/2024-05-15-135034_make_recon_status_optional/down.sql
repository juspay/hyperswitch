-- This file should undo anything in `up.sql`

ALTER TABLE merchant_account 
ALTER COLUMN recon_status SET NOT NULL;