-- This file should undo anything in `up.sql`
ALTER TABLE user_roles RENAME COLUMN last_modified TO last_modified_at;
ALTER TABLE user_roles ALTER COLUMN status TYPE VARCHAR(64) USING (status::text);
DROP TYPE IF EXISTS "UserStatus";
