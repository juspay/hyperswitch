-- Add origin_zip column to address table
ALTER TABLE address
ADD COLUMN  IF NOT EXISTS origin_zip BYTEA;
