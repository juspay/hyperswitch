ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(255);

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR;

ALTER TABLE merchant_account
ALTER COLUMN primary_business_details SET NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_country SET NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_label SET NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_country SET NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_label SET NOT NULL;