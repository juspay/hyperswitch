-- Your SQL goes here
-- Drop the id column as this will be used later as the primary key with a different type
ALTER TABLE merchant_account DROP COLUMN IF EXISTS id;
