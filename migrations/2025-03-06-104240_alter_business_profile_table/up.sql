-- Your SQL goes here
ALTER TABLE business_profile
  ALTER COLUMN webhook_details
  SET DATA TYPE JSON[]
  USING ARRAY[webhook_details]::JSON[];
