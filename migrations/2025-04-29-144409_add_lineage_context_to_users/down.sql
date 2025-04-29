-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN IF EXISTS lineage_context;
