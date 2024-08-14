-- Your SQL goes here
ALTER TABLE customers ADD COLUMN merchant_reference_id VARCHAR(64);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS default_billing_address BYTEA DEFAULT NULL;
ALTER TABLE customers ADD COLUMN IF NOT EXISTS default_shipping_address BYTEA DEFAULT NULL;

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP CONSTRAINT IF EXISTS customers_pkey;

ALTER TABLE customers DROP COLUMN IF EXISTS id;

ALTER TABLE customers ADD COLUMN IF NOT EXISTS id VARCHAR(64);

-- Back filling before making it primary key
UPDATE customers
SET id = customer_id;

ALTER TABLE customers ADD PRIMARY KEY (id);

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP COLUMN customer_id;

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP COLUMN address_id;