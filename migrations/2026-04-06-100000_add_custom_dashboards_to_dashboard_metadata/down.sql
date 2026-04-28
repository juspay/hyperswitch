-- Enum values cannot be removed in PostgreSQL

-- Drop entity_type column
ALTER TABLE dashboard_metadata 
DROP COLUMN IF EXISTS entity_type;

-- Drop index
DROP INDEX IF EXISTS idx_dashboard_metadata_entity_type;
