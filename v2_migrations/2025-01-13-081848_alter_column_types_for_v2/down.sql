-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account
    ALTER COLUMN payment_methods_enabled TYPE JSON [ ];