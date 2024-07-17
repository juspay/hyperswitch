-- Your SQL goes here
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query is necessary for the application to not use id of blocklist in any operation
ALTER TABLE blocklist DROP CONSTRAINT blocklist_pkey;

ALTER TABLE blocklist
ADD PRIMARY KEY (merchant_id, fingerprint_id);
