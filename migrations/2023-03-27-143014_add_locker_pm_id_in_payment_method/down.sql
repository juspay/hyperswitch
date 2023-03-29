-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN locker_payment_method_id;
