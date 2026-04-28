-- Add custom_dashboards enum value
ALTER TYPE "DashboardMetadata" ADD VALUE IF NOT EXISTS 'custom_dashboards';

-- Add entity_type column for scoping (org/merchant/profile level)
-- This is nullable to maintain backward compatibility with existing data
-- Only used for custom_dashboards feature
ALTER TABLE dashboard_metadata 
ADD COLUMN IF NOT EXISTS entity_type VARCHAR(32) NULL;

-- Add index for efficient querying by entity_type
CREATE INDEX IF NOT EXISTS idx_dashboard_metadata_entity_type 
ON dashboard_metadata (entity_type) 
WHERE entity_type IS NOT NULL;
