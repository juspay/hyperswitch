-- This file should undo anything in `up.sql`
ALTER TABLE blocklist DROP CONSTRAINT IF EXISTS blocklist_pkey;

ALTER TABLE blocklist
ADD PRIMARY KEY (id);
