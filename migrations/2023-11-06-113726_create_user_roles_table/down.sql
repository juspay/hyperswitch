-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS user_id_roles_index;
DROP INDEX IF EXISTS user_mid_roles_index;

-- Drop the unique constraint
ALTER TABLE user_roles DROP CONSTRAINT IF EXISTS user_merchant_unique;

-- Drop the table
DROP TABLE user_roles;

-- Drop the user status enum type
DROP TYPE IF EXISTS "UserStatus";