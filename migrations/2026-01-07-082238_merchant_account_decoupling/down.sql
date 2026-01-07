-- This file should undo anything in `up.sql`
UPDATE roles
SET groups = array_remove(groups, 'account_manage')
WHERE
  'account_manage' = ANY (groups)
  AND NOT (
    'webhooks_manage' = ANY (groups)
    OR 'api_keys_manage' = ANY (groups)
  );

UPDATE roles
SET groups = array_remove(groups, 'account_view')
WHERE
  'account_view' = ANY (groups)
  AND NOT (
    'webhooks_view' = ANY (groups)
    OR 'api_keys_view' = ANY (groups)
  );

UPDATE roles
SET groups = array_remove(
               array_remove(groups, 'webhooks_manage'),
               'api_keys_manage'
             )
WHERE
  'account_manage' = ANY (groups)
  AND 'webhooks_manage' = ANY (groups)
  AND 'api_keys_manage' = ANY (groups);

UPDATE roles
SET groups = array_remove(
               array_remove(groups, 'webhooks_view'),
               'api_keys_view'
             )
WHERE
  'account_view' = ANY (groups)
  AND 'webhooks_view' = ANY (groups)
  AND 'api_keys_view' = ANY (groups);