DROP INDEX IF EXISTS dashboard_metadata_index;

CREATE UNIQUE INDEX dashboard_metadata_index
ON dashboard_metadata (COALESCE(user_id, '0'), merchant_id, org_id, data_key);

ALTER TABLE dashboard_metadata DROP COLUMN IF EXISTS profile_id;
