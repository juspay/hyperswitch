ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS fingerprint_secret VARCHAR(128);
