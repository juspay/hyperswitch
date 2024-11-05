-- This file should undo anything in `up.sql`
ALTER TABLE roles DROP CONSTRAINT roles_pkey;

ALTER TABLE roles
ADD PRIMARY KEY (id);
