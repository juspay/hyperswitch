-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS trigger_set_id_organization_name ON organization;

DROP FUNCTION IF EXISTS set_id_organization_name();

DROP TRIGGER IF EXISTS trigger_update_organization_name ON organization;

DROP FUNCTION IF EXISTS update_organization_name();