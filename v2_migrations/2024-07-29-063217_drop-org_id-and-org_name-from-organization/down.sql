-- Alter queries
ALTER TABLE organization
ADD COLUMN org_id VARCHAR(32);

ALTER TABLE organization
ADD COLUMN org_name TEXT;

-- back fill
UPDATE organization
SET org_id = id 
WHERE org_id is NULL;

ALTER TABLE organization
DROP CONSTRAINT organization_pkey_id;

ALTER TABLE organization
ADD CONSTRAINT organization_pkey PRIMARY KEY (org_id);

-- back fill
UPDATE organization
SET org_name = organization_name
WHERE org_name IS NULL AND organization_name IS NOT NULL;