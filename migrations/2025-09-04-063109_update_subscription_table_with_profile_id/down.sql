ALTER TABLE subscription
    DROP CONSTRAINT IF EXISTS subscription_pkey,
    DROP COLUMN IF EXISTS profile_id,
    ADD COLUMN IF NOT EXISTS id SERIAL NOT NULL,
    ADD CONSTRAINT subscription_pkey PRIMARY KEY (id);

ALTER TABLE subscription
    RENAME COLUMN merchant_connector_id TO mca_id;

CREATE UNIQUE INDEX IF NOT EXISTS merchant_subscription_unique_index
    ON subscription (merchant_id, subscription_id);
