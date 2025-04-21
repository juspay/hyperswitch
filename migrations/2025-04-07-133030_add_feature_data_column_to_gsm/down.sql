-- This file should undo anything in `up.sql`
ALTER TABLE gateway_status_map DROP COLUMN IF EXISTS feature_data,
    DROP COLUMN IF EXISTS feature;