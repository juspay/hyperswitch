-- This file should undo anything in `up.sql`
ALTER TABLE payment_link RENAME COLUMN expiry TO fulfilment_time;