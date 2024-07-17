-- This file should undo anything in `up.sql`
-- Use the `merchant_id, fingerprint_id` columns as primary key
-- These are already unique, not null columns
-- So this query should not fail for not null or duplicate value reasons
ALTER TABLE blocklist DROP CONSTRAINT blocklist_pkey;

ALTER TABLE blocklist
ADD PRIMARY KEY (id);
