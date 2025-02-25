-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN version "ApiVersion" DEFAULT 'v1' NOT NULL;
