-- Your SQL goes here
ALTER TABLE customers
ADD COLUMN
IF NOT EXISTS date_of_birth BYTEA DEFAULT NULL;
