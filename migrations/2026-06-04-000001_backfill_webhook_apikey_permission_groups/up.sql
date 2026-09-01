-- Backfill the new webhook/api-key permission groups onto existing roles so the
-- account split preserves the access they had before `account` was shrunk.
UPDATE roles SET groups = array_cat(groups, ARRAY['webhooks_view', 'webhooks_manage', 'api_keys_view', 'api_keys_manage'])
WHERE 'account_manage' = ANY(groups) AND NOT 'webhooks_manage' = ANY(groups);

UPDATE roles SET groups = array_cat(groups, ARRAY['webhooks_view', 'api_keys_view'])
WHERE 'account_view' = ANY(groups) AND NOT 'account_manage' = ANY(groups) AND NOT 'webhooks_view' = ANY(groups);
