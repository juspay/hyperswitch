ALTER TABLE subscription
    DROP CONSTRAINT subscription_pkey,
    DROP COLUMN merchant_connector_id,
    ADD COLUMN merchant_reference_id VARCHAR(128);

ALTER TABLE subscription
    RENAME COLUMN subscription_id TO id;

ALTER TABLE subscription
    ADD PRIMARY KEY (id);
