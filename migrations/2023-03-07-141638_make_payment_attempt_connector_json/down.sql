-- Alter column type to varchar(64) and extract and set the connector
-- name field from the json.
ALTER TABLE payment_attempt
ALTER COLUMN connector TYPE VARCHAR(64)
USING connector->>'routed_through';
