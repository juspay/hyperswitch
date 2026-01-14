-- This file should undo anything in `up.sql`
-- NOTE: The order of these statements is important and must be preserved

-- Roles with account_manage, but without webhooks_manage and api_keys_manage,
-- should move back to NOT having account_manage.
UPDATE roles
SET groups = array_remove(groups, 'account_manage')
WHERE
  'account_manage' = ANY (groups)
  AND NOT (
    'webhooks_manage' = ANY (groups)
    OR 'api_keys_manage' = ANY (groups)
  );

-- Roles with account_view, but without webhooks_view and api_keys_view,
-- should move back to NOT having account_view.
UPDATE roles
SET groups = array_remove(groups, 'account_view')
WHERE
  'account_view' = ANY (groups)
  AND NOT (
    'webhooks_view' = ANY (groups)
    OR 'api_keys_view' = ANY (groups)
  );

-- Roles with account_manage, webhooks_manage and api_keys_manage
-- should retain only account_manage.
UPDATE roles
SET groups = array_remove(
               array_remove(groups, 'webhooks_manage'),
               'api_keys_manage'
             )
WHERE
  'account_manage' = ANY (groups)
  AND 'webhooks_manage' = ANY (groups)
  AND 'api_keys_manage' = ANY (groups);

-- Roles with account_view, webhooks_view and api_keys_view
-- should retain only account_view.
UPDATE roles
SET groups = array_remove(
               array_remove(groups, 'webhooks_view'),
               'api_keys_view'
             )
WHERE
  'account_view' = ANY (groups)
  AND 'webhooks_view' = ANY (groups)
  AND 'api_keys_view' = ANY (groups);
