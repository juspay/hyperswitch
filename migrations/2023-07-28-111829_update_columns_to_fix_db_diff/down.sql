ALTER TABLE dispute
ALTER COLUMN payment_id TYPE VARCHAR(255);

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR;