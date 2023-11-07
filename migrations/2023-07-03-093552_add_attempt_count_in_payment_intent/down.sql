-- This file should undo anything in `up.sql`
ALTER TABLE PAYMENT_INTENT DROP COLUMN attempt_count;
