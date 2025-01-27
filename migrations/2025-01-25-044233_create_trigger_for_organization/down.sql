-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS trigger_set_org_id_org_name ON organization;

DROP FUNCTION IF EXISTS set_org_id_org_name();

DROP TRIGGER IF EXISTS trigger_update_org_name ON organization;

DROP FUNCTION IF EXISTS update_org_name();