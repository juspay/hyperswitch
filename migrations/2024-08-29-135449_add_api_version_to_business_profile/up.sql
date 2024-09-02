-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN api_version "ApiVersion" DEFAULT 'v1' NOT NULL;
