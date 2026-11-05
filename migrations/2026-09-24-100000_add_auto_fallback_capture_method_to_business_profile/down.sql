-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN IF EXISTS auto_fallback_capture_method;
