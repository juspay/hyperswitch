-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS role_id_index;
DROP INDEX IF EXISTS roles_merchant_org_index;

DROP TABLE IF EXISTS roles;
DROP TYPE "RoleScope";