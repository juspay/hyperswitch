-- This file should undo anything in `up.sql`
ALTER TABLE user_roles DROP COLUMN IF EXISTS tenant_id;