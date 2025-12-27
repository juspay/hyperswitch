-- Your SQL goes here
ALTER TABLE customers
ADD COLUMN
IF NOT EXISTS customer_document_number BYTEA DEFAULT NULL;