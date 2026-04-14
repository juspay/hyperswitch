ALTER TABLE dashboard_metadata
    ADD COLUMN profile_id VARCHAR(64);

ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'payment_views';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'refund_views';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'customer_views';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'dispute_views';
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'payout_views';

DROP INDEX IF EXISTS dashboard_metadata_index;

CREATE UNIQUE INDEX dashboard_metadata_index_v2
ON dashboard_metadata (
    COALESCE(user_id, '0'),
    merchant_id,
    org_id,
    COALESCE(profile_id, '0'),
    data_key
);

ALTER INDEX dashboard_metadata_index_v2 RENAME TO dashboard_metadata_index;
