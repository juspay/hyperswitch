-- Your SQL goes here
ALTER TABLE user_roles ALTER COLUMN org_id DROP NOT NULL;
ALTER TABLE user_roles ALTER COLUMN merchant_id DROP NOT NULL;
ALTER TABLE user_roles ADD COLUMN profile_id VARCHAR(255);
ALTER TABLE user_roles ADD COLUMN entity_id VARCHAR(255);
ALTER TABLE user_roles ADD COLUMN entity_type VARCHAR(64);
ALTER TABLE user_roles ADD COLUMN version VARCHAR(8) DEFAULT 'v1';
