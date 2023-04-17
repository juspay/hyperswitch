ALTER TABLE merchant_connector_account
ADD COLUMN connector_label VARCHAR(255),
    ADD COLUMN business_country VARCHAR(2) DEFAULT 'US' NOT NULL,
    ADD COLUMN business_label VARCHAR(255) DEFAULT 'default' NOT NULL;

-- To backfill, use `US` as default country and `default` as the business_label
UPDATE merchant_connector_account AS m
SET connector_label = concat(
        m.connector_name,
        '_',
        'US',
        '_',
        'default'
    );

ALTER TABLE merchant_connector_account
ALTER COLUMN connector_label
SET NOT NULL,
    ALTER COLUMN business_country DROP DEFAULT,
    ALTER COLUMN business_label DROP DEFAULT;

DROP INDEX merchant_connector_account_merchant_id_connector_name_index;

CREATE UNIQUE INDEX merchant_connector_account_merchant_id_connector_label_index ON merchant_connector_account (merchant_id, connector_label);
