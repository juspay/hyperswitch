-- Backfill
UPDATE organization
SET id = org_id 
WHERE id is NULL;

UPDATE organization
SET organization_name = org_name
WHERE organization_name IS NULL AND org_name IS NOT NULL;

-- Alter queries
ALTER TABLE organization
DROP COLUMN org_id;

ALTER TABLE organization
DROP COLUMN org_name;

ALTER TABLE organization
ADD CONSTRAINT organization_pkey_id PRIMARY KEY (id);