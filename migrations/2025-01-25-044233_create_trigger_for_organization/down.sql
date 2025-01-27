-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS before_insert_trigger ON organization;

DROP FUNCTION IF EXISTS set_not_null_field();
