-- This file should undo anything in `up.sql`

-- Drop the table
DROP TABLE IF EXISTS user_roles;

-- Drop the user status enum type
DROP TYPE IF EXISTS "UserStatus";