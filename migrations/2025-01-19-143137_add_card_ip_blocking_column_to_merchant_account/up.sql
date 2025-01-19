-- Your SQL goes here

ALTER TABLE merchant_account
        ADD COLUMN card_ip_blocking BOOL NOT NULL DEFAULT FALSE;