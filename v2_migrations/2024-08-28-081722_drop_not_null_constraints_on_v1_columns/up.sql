-- Drop not null constaint on org_id in orgnaization table
ALTER TABLE organization ALTER COLUMN org_id DROP NOT NULL;