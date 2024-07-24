-- Your SQL goes here
ALTER TABLE organization
RENAME COLUMN org_id TO id;
ALTER TABLE organization
RENAME COLUMN org_name TO organization_name;