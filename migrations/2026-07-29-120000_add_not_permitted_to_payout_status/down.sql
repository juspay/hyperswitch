-- Postgres does not support removing a value from an enum type without
-- recreating it; this is a no-op to keep the migration reversible in form.
SELECT 1;
