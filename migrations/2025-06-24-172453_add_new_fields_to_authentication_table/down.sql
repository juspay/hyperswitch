-- This file should undo anything in `up.sql`
ALTER TABLE authentication 
DROP COLUMN IF EXISTS billing_address,
DROP COLUMN IF EXISTS shipping_address,
DROP COLUMN IF EXISTS browser_info,
DROP COLUMN IF EXISTS email;
