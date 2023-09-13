ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS applepay_verified_domains text[];
