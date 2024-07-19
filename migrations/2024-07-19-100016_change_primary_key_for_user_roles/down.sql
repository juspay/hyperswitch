-- This file should undo anything in `up.sql`
ALTER TABLE user_roles DROP CONSTRAINT user_roles_pkey;

ALTER TABLE user_roles
ADD PRIMARY KEY (id);
