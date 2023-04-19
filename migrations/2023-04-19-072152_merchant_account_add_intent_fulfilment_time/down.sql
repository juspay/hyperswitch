-- This file should undo anything in `up.sql`

ALTER TABLE merchant_account DROP COLUMN IF EXISTS intent_fulfillment_time; 
