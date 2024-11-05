-- This file should undo anything in `up.sql`
ALTER TABLE organization
DROP COLUMN organization_details,
DROP COLUMN metadata,
DROP created_at,
DROP modified_at;