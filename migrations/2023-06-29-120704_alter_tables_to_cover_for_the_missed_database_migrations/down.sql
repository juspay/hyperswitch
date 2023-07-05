ALTER TABLE file_metadata
    ADD COLUMN connector_label VARCHAR(255);

ALTER TABLE merchant_account
    ALTER COLUMN merchant_name DROP NOT NULL,
    ALTER COLUMN primary_business_details SET DEFAULT '[{""country"": ""US"", ""business"": ""default""}]';

ALTER TABLE merchant_key_store
    ALTER COLUMN merchant_id TYPE VARCHAR(64);

ALTER TABLE payment_intent
    ALTER COLUMN metadata DROP DEFAULT,
    DROP COLUMN meta_data;

ALTER TABLE payment_methods
    ALTER COLUMN payment_method_type TYPE VARCHAR;