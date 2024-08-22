-- Your SQL goes here
ALTER TABLE payouts
ADD COLUMN sec_code VARCHAR NULL,
ADD COLUMN account_type VARCHAR NULL;
