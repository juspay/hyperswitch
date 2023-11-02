ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(255);

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR;

ALTER TABLE merchant_account
ALTER COLUMN primary_business_details SET DEFAULT '[{"country": "US", "business": "default"}]';