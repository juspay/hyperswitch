-- Your SQL goes here
-- First drop the primary key of merchant_account
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;

-- Create new primary key
ALTER TABLE merchant_account
ADD PRIMARY KEY (merchant_id);

-- Drop the id column as this will be used later as the primary key with a different type
ALTER TABLE merchant_account DROP COLUMN IF EXISTS id;
