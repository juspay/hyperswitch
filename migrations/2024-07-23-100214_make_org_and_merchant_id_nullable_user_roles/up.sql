-- Your SQL goes here
ALTER TABLE user_roles DROP CONSTRAINT user_roles_pkey;
ALTER TABLE user_roles ADD PRIMARY KEY (id);

ALTER TABLE user_roles ALTER COLUMN org_id DROP NOT NULL;
ALTER TABLE user_roles ALTER COLUMN merchant_id DROP NOT NULL;

ALTER TABLE user_roles ADD COLUMN profile_id VARCHAR(255);
ALTER TABLE user_roles ADD COLUMN entity_id VARCHAR(255);
ALTER TABLE user_roles ADD COLUMN entity_type VARCHAR(64);

CREATE TYPE "UserRoleVersion" AS ENUM('v1', 'v2');
ALTER TABLE user_roles ADD COLUMN version "UserRoleVersion" DEFAULT 'v1' NOT NULL;
