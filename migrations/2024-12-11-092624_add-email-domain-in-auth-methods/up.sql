-- Your SQL goes here
ALTER TABLE user_authentication_methods ADD COLUMN email_domain VARCHAR(64);
UPDATE user_authentication_methods SET email_domain = auth_id WHERE email_domain IS NULL;
ALTER TABLE user_authentication_methods ALTER COLUMN email_domain SET NOT NULL;

CREATE INDEX email_domain_index ON user_authentication_methods (email_domain);
