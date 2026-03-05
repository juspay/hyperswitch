-- Your SQL goes here
UPDATE roles
SET groups = array_replace(groups, 'merchant_details_view', 'account_view')
WHERE 'merchant_details_view' = ANY(groups);

UPDATE roles
SET groups = array_replace(groups, 'merchant_details_manage', 'account_manage')
WHERE 'merchant_details_manage' = ANY(groups);

UPDATE roles
SET groups = array_replace(groups, 'organization_manage', 'account_manage')
WHERE 'organization_manage' = ANY(groups);