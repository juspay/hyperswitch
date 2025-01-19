-- This file should undo anything in `up.sql`

ALTER TABLE merchant_account
        DROP COLUMN card_ip_blocking;