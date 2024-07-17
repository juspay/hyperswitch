-- This file should undo anything in `up.sql`
ALTER TABLE user_roles ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE user_roles ALTER COLUMN merchant_id SET NOT NULL;
ALTER TABLE user_roles DROP COLUMN profile_id;
ALTER TABLE user_roles DROP COLUMN entity_id;
ALTER TABLE user_roles DROP COLUMN entity_type;
ALTER TABLE user_roles DROP COLUMN version;
DROP TYPE IF EXISTS "UserRoleVersion";