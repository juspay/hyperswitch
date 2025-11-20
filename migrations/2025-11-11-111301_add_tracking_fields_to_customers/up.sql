-- Your SQL goes here
-- Add created_by column to customers table for tracking the creator/origin of the record
ALTER TABLE customers ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);

-- Add last_modified_by column to customers table for tracking who last modified the record
ALTER TABLE customers ADD COLUMN IF NOT EXISTS last_modified_by VARCHAR(255);
