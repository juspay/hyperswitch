ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(255);

ALTER TABLE merchant_key_store
ALTER COLUMN created_at DROP DEFAULT;

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR;