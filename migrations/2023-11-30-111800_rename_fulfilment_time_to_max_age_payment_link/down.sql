-- This file should undo anything in `up.sql`
ALTER TABLE payment_link RENAME COLUMN max_age TO fulfilment_time;