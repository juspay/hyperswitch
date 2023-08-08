ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(64);

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR(255);

ALTER TABLE merchant_account
ALTER COLUMN primary_business_details DROP NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_country DROP NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_label DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_country DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_label DROP NOT NULL;