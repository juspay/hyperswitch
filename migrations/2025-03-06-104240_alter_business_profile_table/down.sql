-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
  ALTER COLUMN webhook_details
  SET DATA TYPE JSON
