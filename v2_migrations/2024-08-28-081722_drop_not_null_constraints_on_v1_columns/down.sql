-- This file should undo anything in `up.sql`
ALTER TABLE organization ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE organization ADD PRIMARY KEY (org_id);