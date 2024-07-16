-- This file should undo anything in `up.sql`

ALTER TABLE payment_methods DROP COLUMN IF EXISTS locker_id;
