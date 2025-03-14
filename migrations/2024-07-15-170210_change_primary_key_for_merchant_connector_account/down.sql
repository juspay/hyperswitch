-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

ALTER TABLE merchant_connector_account
ADD PRIMARY KEY (id);
