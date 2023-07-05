ALTER TABLE file_metadata
    DROP COLUMN connector_label;

ALTER TABLE merchant_account
    ALTER COLUMN merchant_name SET NOT NULL,
    ALTER COLUMN primary_business_details DROP DEFAULT;

ALTER TABLE merchant_key_store
    ALTER COLUMN merchant_id TYPE VARCHAR(255);

ALTER TABLE payment_intent
    ALTER COLUMN metadata DEFAULT '{}'::JSONB,
    ADD COLUMN meta_data JSONB;

ALTER TABLE payment_methods
    ALTER COLUMN payment_method_type TYPE VARCHAR(64);

ALTER TABLE prod_intent
    ALTER COLUMN is_completed NULL;