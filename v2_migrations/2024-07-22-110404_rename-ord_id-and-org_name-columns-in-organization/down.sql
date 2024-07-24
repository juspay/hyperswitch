-- This file should undo anything in `up.sql`
-- Your SQL goes here
ALTER TABLE organization
RENAME COLUMN id TO org_id;
ALTER TABLE organization
RENAME COLUMN organization_name TO org_name;