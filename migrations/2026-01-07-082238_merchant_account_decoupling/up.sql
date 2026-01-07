-- Your SQL goes here
-- NOTE: The order of these statements is important and must be preserved
UPDATE roles
SET groups = array_cat(groups, ARRAY['webhooks_manage', 'api_keys_manage'])
WHERE 'account_manage' = ANY(groups);

UPDATE roles
SET groups = array_cat(groups, ARRAY['webhooks_view', 'api_keys_view'])
WHERE 'account_view' = ANY(groups)
  AND NOT 'account_manage' = ANY(groups);

UPDATE roles
SET groups = array_append(groups, 'account_manage')
WHERE
  'account_manage' <> ALL (groups)
  AND EXISTS (
    SELECT 1
    FROM unnest(groups) AS g
    WHERE g LIKE '%_manage'
  );

UPDATE roles
SET groups = array_append(groups, 'account_view')
WHERE
  'account_view' <> ALL (groups)
  AND NOT EXISTS (
    SELECT 1
    FROM unnest(groups) AS g
    WHERE g LIKE '%_manage'
  )
  AND EXISTS (
    SELECT 1
    FROM unnest(groups) AS g
    WHERE g LIKE '%_view'
  );
