-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS org_id_auth_methods_index;
DROP TABLE IF EXISTS org_authentication_methods;

DROP TYPE IF EXISTS "AuthMethod";
