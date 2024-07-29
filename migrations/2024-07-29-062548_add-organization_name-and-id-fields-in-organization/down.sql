-- This file should undo anything in `up.sql`
ALTER TABLE organization
DROP COLUMN id;
ALTER TABLE organization
DROP COLUMN organization_name;