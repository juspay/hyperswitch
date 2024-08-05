-- Your SQL goes here
-- Adding a new column called `id` which will be the new primary key for v2
-- Note that even though this will be the new primary key, the v1 application would still fill in null values
ALTER TABLE merchant_account
ADD COLUMN id VARCHAR(64);
