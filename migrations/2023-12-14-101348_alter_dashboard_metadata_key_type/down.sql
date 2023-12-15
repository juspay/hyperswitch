-- This file should undo anything in `up.sql`
ALTER TABLE dashboard_metadata ALTER COLUMN data_key TYPE VARCHAR(64);
DROP TYPE IF EXISTS "DashboardMetadata";
