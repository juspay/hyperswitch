-- Account resource is being pulled out of every parent group except `account`,
-- so roles that relied on the implicit Account access need it spelled out.
-- Only the groups whose parent still bundled `Resource::Account` count here;
-- theme/internal/webhook/api-key groups never carried it and are left out.
UPDATE roles SET groups = array_append(groups, 'account_manage')
WHERE 'account_manage' <> ALL(groups)
  AND groups && ARRAY[
    'operations_manage', 'connectors_manage', 'workflows_manage', 'users_manage',
    'recon_sources_manage', 'recon_exceptions_manage', 'recon_transactions_manage', 'recon_rules_manage'
  ];

UPDATE roles SET groups = array_append(groups, 'account_view')
WHERE 'account_view' <> ALL(groups)
  AND groups && ARRAY[
    'operations_view', 'operations_manage', 'connectors_view', 'connectors_manage',
    'workflows_view', 'workflows_manage', 'analytics_view', 'users_view', 'users_manage',
    'recon_sources_view', 'recon_sources_manage', 'recon_exceptions_view', 'recon_exceptions_manage',
    'recon_transactions_view', 'recon_transactions_manage', 'recon_rules_view', 'recon_rules_manage',
    'account_manage'
  ];
