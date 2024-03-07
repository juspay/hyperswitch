-- This file should undo anything in `up.sql`

ALTER TABLE payment_methods DROP COLUMN network_transaction_id DEFAULT NOT NULL;