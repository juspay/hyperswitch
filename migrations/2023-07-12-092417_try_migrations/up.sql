ALTER TABLE merchant_account
    ALTER COLUMN merchant_name SET NOT NULL,
    ALTER COLUMN primary_business_details DROP DEFAULT;

ALTER TABLE merchant_key_store
    ALTER COLUMN merchant_id TYPE VARCHAR(255);

ALTER TABLE payment_intent
    ALTER COLUMN metadata SET DEFAULT '{}'::JSONB;

ALTER TABLE payment_methods
    ALTER COLUMN payment_method_type TYPE VARCHAR(64); 