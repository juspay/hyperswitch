-- Your SQL goes here
ALTER TABLE address
ADD COLUMN customer_id VARCHAR(255) NOT NULL,
ADD COLUMN merchant_id VARCHAR(255) NOT NULL;

CREATE INDEX address_customer_id_merchant_id_index ON address (customer_id, merchant_id);

ALTER TABLE customers DROP COLUMN address;