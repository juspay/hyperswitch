ALTER TABLE address
    ALTER COLUMN customer_id DROP DEFAULT,
    ALTER COLUMN merchant_id DROP DEFAULT;

ALTER TABLE dispute
    ALTER COLUMN challenge_required_by TYPE TIMESTAMP,
    ALTER COLUMN connector_created_at TYPE TIMESTAMP,
    ALTER COLUMN connector_updated_at TYPE TIMESTAMP;

ALTER TABLE events
    ALTER COLUMN event_id TYPE VARCHAR(64),
    ALTER COLUMN intent_reference_id TYPE VARCHAR(64),
    ALTER COLUMN primary_object_id TYPE VARCHAR(64);

ALTER TABLE feedbacks
    ALTER COLUMN created_at NOT NULL;

ALTER TABLE file_metadata
    ADD COLUMN connector_label VARCHAR(255);

ALTER TABLE merchant_account
    DROP COLUMN api_key,
    ALTER COLUMN merchant_name NULL,
    ALTER COLUMN locker_id DROP DEFAULT,
    ALTER COLUMN primary_business_details NOT NULL DEFAULT '{""country"": [""US""], ""business"": [""default""]}';

ALTER TABLE merchant_connector_account
    ALTER COLUMN business_country NOT NULL,
    ALTER COLUMN business_label NOT NULL;

ALTER TABLE merchant_key_store
    ALTER COLUMN merchant_id TYPE VARCHAR(64);

ALTER TABLE payment_intent
    ALTER COLUMN metadata DROP DEFAULT,
    ALTER COLUMN business_country NOT NULL,
    ALTER COLUMN business_label NOT NULL,
    DROP COLUMN meta_data;

ALTER TABLE payment_methods
    ALTER COLUMN payment_method_type TYPE VARCHAR;

ALTER TABLE prod_intent
    ALTER COLUMN is_completed NOT NULL;