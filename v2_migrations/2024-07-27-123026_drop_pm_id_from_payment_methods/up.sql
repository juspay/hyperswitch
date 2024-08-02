-- Your SQL goes here
UPDATE payment_methods SET id = payment_method_id;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_id;