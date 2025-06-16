-- Your SQL goes here
-- Incomplete migration, also run migrations/2024-12-13-080558_entity-id-backfill-for-user-roles
UPDATE user_roles
SET
    entity_type = CASE
        WHEN role_id = 'org_admin' THEN 'organization'
        ELSE 'merchant'
    END
WHERE
    version = 'v1'
    AND entity_type IS NULL;