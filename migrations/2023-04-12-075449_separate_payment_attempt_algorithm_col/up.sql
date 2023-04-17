-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN straight_through_algorithm JSONB;

UPDATE payment_attempt SET straight_through_algorithm = connector->'algorithm'
WHERE connector->>'algorithm' IS NOT NULL;

ALTER TABLE payment_attempt
ALTER COLUMN connector TYPE VARCHAR(64)
USING connector->>'routed_through';
