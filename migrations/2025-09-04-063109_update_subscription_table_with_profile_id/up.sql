DROP INDEX IF EXISTS merchant_subscription_unique_index;

ALTER TABLE subscription
    DROP CONSTRAINT IF EXISTS subscription_pkey,
    DROP COLUMN IF EXISTS id,
    ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64) NOT NULL,
    ADD CONSTRAINT subscription_pkey PRIMARY KEY (subscription_id, merchant_id);

ALTER TABLE subscription
    RENAME COLUMN mca_id TO merchant_connector_id;
