-- Your SQL goes here

DROP INDEX IF EXISTS role_name_org_id_org_scope_index;

DROP INDEX IF EXISTS role_name_merchant_id_merchant_scope_index;

DROP INDEX IF EXISTS roles_merchant_org_index;

CREATE INDEX roles_merchant_org_index ON roles (
    org_id,
    merchant_id,
    profile_id
);