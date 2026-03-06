ALTER TABLE dashboard_metadata
    ADD COLUMN profile_id VARCHAR(64);

ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'payments';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'refunds';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'customers';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'disputes';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'payouts';

CREATE UNIQUE INDEX dashboard_metadata_index_v2
ON dashboard_metadata (
    COALESCE(user_id, '0'),
    merchant_id,
    org_id,
    COALESCE(profile_id, '0'),
    data_key
);

DROP INDEX IF EXISTS dashboard_metadata_index;

ALTER INDEX dashboard_metadata_index_v2 RENAME TO dashboard_metadata_index;
