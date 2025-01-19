-- Your SQL goes here

ALTER TABLE merchant_account
        ADD COLUMN guest_user_card_blocking BOOL NOT NULL DEFAULT FALSE;