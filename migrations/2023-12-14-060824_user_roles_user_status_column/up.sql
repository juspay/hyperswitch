-- Your SQL goes here
ALTER TABLE user_roles RENAME COLUMN last_modified_at TO last_modified;
CREATE TYPE "UserStatus" AS ENUM ('active', 'invitation_sent');
ALTER TABLE user_roles ALTER COLUMN status TYPE "UserStatus" USING (status::"UserStatus");
