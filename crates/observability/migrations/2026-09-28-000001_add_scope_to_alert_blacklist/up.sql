ALTER TABLE alert_blacklist
    ADD COLUMN IF NOT EXISTS scope JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE alert_blacklist
    DROP CONSTRAINT alert_blacklist_pkey;

ALTER TABLE alert_blacklist
    ADD PRIMARY KEY (rule_id, merchant_id, profile_id, scope);
