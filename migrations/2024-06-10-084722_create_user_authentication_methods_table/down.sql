-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS auth_id_index;
DROP INDEX IF EXISTS owner_id_index;
DROP TABLE IF EXISTS user_authentication_methods;
