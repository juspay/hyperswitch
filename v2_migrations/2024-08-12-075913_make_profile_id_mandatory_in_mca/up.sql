-- Your SQL goes here
-- This migration is to make profile_id mandatory in mca table
ALTER TABLE merchant_connector_account ALTER COLUMN profile_id SET NOT NULL;
