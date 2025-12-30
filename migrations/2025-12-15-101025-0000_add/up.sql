-- Your SQL goes here
ALTER TABLE customers
ADD COLUMN
IF NOT EXISTS document_number BYTEA DEFAULT NULL;