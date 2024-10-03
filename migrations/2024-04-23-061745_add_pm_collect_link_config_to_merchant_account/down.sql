ALTER TABLE merchant_account
DROP COLUMN IF EXISTS pm_collect_link_config;

ALTER TABLE business_profile
DROP COLUMN IF EXISTS payout_link_config;