-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP COLUMN IF EXISTS merchant_reference_id;
ALTER TABLE customers DROP COLUMN IF EXISTS default_billing_address;
ALTER TABLE customers DROP COLUMN IF EXISTS default_shipping_address;

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP CONSTRAINT customers_pkey;
ALTER TABLE customers DROP COLUMN IF EXISTS id;
ALTER TABLE customers ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE customers ADD COLUMN customer_id VARCHAR(64);

-- Back filling before making it primary key
UPDATE customers
SET customer_id = id;


ALTER TABLE customers ADD PRIMARY KEY (merchant_id, customer_id);

ALTER TABLE customers ADD COLUMN address_id VARCHAR(64);
