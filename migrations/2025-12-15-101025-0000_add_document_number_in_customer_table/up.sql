-- Your SQL goes here
ALTER TABLE customers
ADD COLUMN
IF NOT EXISTS document_details BYTEA DEFAULT NULL;