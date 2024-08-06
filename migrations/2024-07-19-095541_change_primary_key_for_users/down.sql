-- This file should undo anything in `up.sql`
ALTER TABLE users DROP CONSTRAINT users_pkey;

ALTER TABLE users
ADD PRIMARY KEY (id);
