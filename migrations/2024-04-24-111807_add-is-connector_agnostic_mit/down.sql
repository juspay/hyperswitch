-- This file should undo anything in `up.sql`

ALTER TABLE business_profile DROP COLUMN IF EXISTS is_connector_agnostic_mit_enabled;