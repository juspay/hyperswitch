ALTER TABLE address
    ALTER COLUMN customer_id DEFAULT 'dummy_cust',
    ALTER COLUMN merchant_id DEFAULT 'dummy';

ALTER TABLE dispute
    ALTER COLUMN challenge_required_by TYPE VARCHAR(255),
    ALTER COLUMN connector_created_at TYPE VARCHAR(255),
    ALTER COLUMN connector_updated_at TYPE VARCHAR(255);

ALTER TABLE events
    ALTER COLUMN event_id TYPE VARCHAR(255),
    ALTER COLUMN intent_reference_id TYPE VARCHAR(255),
    ALTER COLUMN primary_object_id TYPE VARCHAR(255);

ALTER TABLE feedbacks
    ALTER COLUMN created_at NULL;

ALTER TABLE file_metadata
    DROP COLUMN connector_label;

ALTER TABLE merchant_account
    ADD COLUMN api_key VARCHAR(128),
    ALTER COLUMN merchant_name NOT NULL,
    ALTER COLUMN locker_id DEFAULT 'm0010',
    ALTER COLUMN primary_business_details NULL DEFAULT '[{""country"": ""US"", ""business"": ""default""}]';

ALTER TABLE merchant_connector_account
    ALTER COLUMN business_country NULL,
    ALTER COLUMN business_label NULL;

ALTER TABLE merchant_key_store
    ALTER COLUMN merchant_id TYPE VARCHAR(255);

ALTER TABLE payment_intent
    ALTER COLUMN metadata DEFAULT '{}'::JSONB,
    ALTER COLUMN business_country NULL,
    ALTER COLUMN business_label NULL,
    ADD COLUMN meta_data JSONB;

ALTER TABLE payment_methods
    ALTER COLUMN payment_method_type TYPE VARCHAR(64);

ALTER TABLE prod_intent
    ALTER COLUMN is_completed NULL;