-- This file should undo anything in `up.sql`

ALTER TABLE business_profile
DROP COLUMN customer_id_blocking_threshold;