-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
ALTER COLUMN business_country
SET NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_label
SET NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_country
SET NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_label
SET NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN connector_label
SET NOT NULL;
