-- Your SQL goes here
-- NOTE: The order of these statements is important and must be preserved

-- Roles with account_manage must also manage webhooks and API keys
UPDATE roles
SET groups = array_cat(groups, ARRAY['webhooks_manage', 'api_keys_manage'])
WHERE 'account_manage' = ANY(groups);

-- Roles with account_view (but not account_manage)
-- must also have view access to webhooks and API keys
UPDATE roles
SET groups = array_cat(groups, ARRAY['webhooks_view', 'api_keys_view'])
WHERE 'account_view' = ANY(groups)
  AND NOT 'account_manage' = ANY(groups);

-- Roles with ANY *_manage permission
-- but without account_manage, must explicitly have account_manage
UPDATE roles
SET groups = array_append(groups, 'account_manage')
WHERE
  'account_manage' <> ALL (groups)
  AND EXISTS (
    SELECT 1
    FROM unnest(groups) AS g
    WHERE g LIKE '%_manage'
  );

-- Roles without account_view must explicitly have account_view
UPDATE roles
SET groups = array_append(groups, 'account_view')
WHERE 'account_view' <> ALL (groups);
