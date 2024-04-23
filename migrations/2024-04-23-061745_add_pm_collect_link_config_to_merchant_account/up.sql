ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS pm_collect_link_config JSONB NULL;