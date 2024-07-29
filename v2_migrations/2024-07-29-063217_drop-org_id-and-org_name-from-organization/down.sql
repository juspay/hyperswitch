-- Alter queries
ALTER TABLE organization
DROP CONSTRAINT pk_id;

ALTER TABLE organization
ADD COLUMN org_id VARCHAR(32) PRIMARY KEY NOT NULL;

ALTER TABLE organization
ADD COLUMN org_name TEXT;