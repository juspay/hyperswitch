-- Merchant-level Offer Engine credentials (encrypted), used when the resolved
-- credential source is `merchant`.
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS offer_engine_config BYTEA;
