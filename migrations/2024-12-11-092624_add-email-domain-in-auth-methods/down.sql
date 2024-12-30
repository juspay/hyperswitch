-- This file should undo anything in `up.sql`
DROP INDEX email_domain_index;
ALTER TABLE user_authentication_methods DROP COLUMN email_domain;
