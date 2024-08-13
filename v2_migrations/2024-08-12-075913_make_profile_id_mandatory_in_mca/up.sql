-- Your SQL goes here

-- Backfill the profile_id column with the default value for the one record which was created before the profile_id was added in create seed data
UPDATE merchant_connector_account SET profile_id = 'default_profile_id' WHERE profile_id IS NULL;

-- This migration is to make profile_id mandatory in mca table
ALTER TABLE merchant_connector_account ALTER COLUMN profile_id SET NOT NULL;
