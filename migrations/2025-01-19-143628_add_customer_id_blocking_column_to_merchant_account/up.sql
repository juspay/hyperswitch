-- Your SQL goes here

ALTER TABLE merchant_account
        ADD COLUMN customer_id_blocking BOOL NOT NULL DEFAULT FALSE;