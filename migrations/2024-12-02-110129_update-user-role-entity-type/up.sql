-- Your SQL goes here
UPDATE user_roles
SET
    entity_type = CASE
        WHEN role_id = 'org_admin' THEN 'organization'
        ELSE 'merchant'
    END
WHERE
    version = 'v1'
    AND entity_type IS NULL;