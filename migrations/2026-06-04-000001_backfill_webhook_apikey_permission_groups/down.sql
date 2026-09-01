-- Strip the webhook/api-key permission groups added by the backfill.
UPDATE roles SET groups = array_remove(groups, 'webhooks_manage') WHERE 'webhooks_manage' = ANY(groups);
UPDATE roles SET groups = array_remove(groups, 'api_keys_manage') WHERE 'api_keys_manage' = ANY(groups);
UPDATE roles SET groups = array_remove(groups, 'webhooks_view') WHERE 'webhooks_view' = ANY(groups);
UPDATE roles SET groups = array_remove(groups, 'api_keys_view') WHERE 'api_keys_view' = ANY(groups);
