-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;

ALTER TABLE merchant_account
ADD PRIMARY KEY (id);
