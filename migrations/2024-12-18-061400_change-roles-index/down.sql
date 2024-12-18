-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX role_name_org_id_org_scope_index ON roles (org_id, role_name)
WHERE
    scope = 'organization';

CREATE UNIQUE INDEX role_name_merchant_id_merchant_scope_index ON roles (merchant_id, role_name)
WHERE
    scope = 'merchant';

DROP INDEX IF EXISTS roles_merchant_org_index;