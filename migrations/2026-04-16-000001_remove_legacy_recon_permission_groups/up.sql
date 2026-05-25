UPDATE roles
SET groups = array_remove(groups, 'recon_ops_view')
WHERE 'recon_ops_view' = ANY(groups);

UPDATE roles
SET groups = array_remove(groups, 'recon_ops_manage')
WHERE 'recon_ops_manage' = ANY(groups);

UPDATE roles
SET groups = array_remove(groups, 'recon_reports_view')
WHERE 'recon_reports_view' = ANY(groups);

UPDATE roles
SET groups = array_remove(groups, 'recon_reports_manage')
WHERE 'recon_reports_manage' = ANY(groups);
