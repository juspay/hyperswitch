-- Your SQL goes here
ALTER TABLE payment_methods
ADD COLUMN locker_payment_method_id VARCHAR DEFAULT NULL;
