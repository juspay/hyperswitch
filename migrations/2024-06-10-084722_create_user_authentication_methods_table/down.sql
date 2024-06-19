-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS auth_id;
DROP INDEX IF EXISTS owner_id;
DROP TABLE IF EXISTS user_authentication_methods;
