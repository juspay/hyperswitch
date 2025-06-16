-- Your SQL goes here
UPDATE user_roles
SET
    entity_id = CASE
        WHEN role_id = 'org_admin' THEN org_id
        ELSE merchant_id
    END
WHERE
    version = 'v1'
    AND entity_id IS NULL;