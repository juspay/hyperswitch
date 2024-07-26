-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP CONSTRAINT customers_pkey;
ALTER TABLE customers DROP COLUMN IF EXISTS id;
ALTER TABLE customers ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE customers ADD PRIMARY KEY (merchant_id, customer_id);
