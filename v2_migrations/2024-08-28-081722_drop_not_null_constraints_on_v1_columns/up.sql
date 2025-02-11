-- Drop not null constraint on org_id in orgnaization table
ALTER TABLE organization DROP CONSTRAINT organization_pkey;
ALTER TABLE organization ALTER COLUMN org_id DROP NOT NULL;