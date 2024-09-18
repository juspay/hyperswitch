-- Your SQL goes here
ALTER TABLE organization
ADD CONSTRAINT organization_organization_name_key UNIQUE (organization_name);