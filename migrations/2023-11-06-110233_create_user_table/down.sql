-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS user_id_index;
DROP INDEX IF EXISTS user_email_index;

DROP TABLE users;