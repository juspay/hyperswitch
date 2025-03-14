-- Your SQL goes here

ALTER TABLE payment_attempt ADD COLUMN card_network VARCHAR(32);
UPDATE payment_attempt
SET card_network = (payment_method_data -> 'card' -> 'card_network')::VARCHAR(32);