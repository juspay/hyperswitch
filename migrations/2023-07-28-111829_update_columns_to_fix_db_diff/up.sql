ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(64);

ALTER TABLE merchant_key_store
ALTER COLUMN created_at SET DEFAULT now();

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR(64);