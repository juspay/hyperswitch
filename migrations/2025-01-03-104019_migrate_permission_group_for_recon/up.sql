UPDATE roles
SET groups = array_replace(groups, 'recon_ops', 'recon_ops_manage')
WHERE 'recon_ops' = ANY(groups);
