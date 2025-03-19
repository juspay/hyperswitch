-- Your SQL goes here
ALTER TABLE merchant_account
  ALTER COLUMN webhook_details
  SET DATA TYPE JSON[]
  USING ARRAY[webhook_details]::JSON[];