ALTER TABLE subscription
    DROP CONSTRAINT subscription_pkey,
    DROP COLUMN merchant_reference_id;

ALTER TABLE subscription
    RENAME COLUMN id TO subscription_id;

ALTER TABLE subscription
    ADD PRIMARY KEY (subscription_id, merchant_id);
