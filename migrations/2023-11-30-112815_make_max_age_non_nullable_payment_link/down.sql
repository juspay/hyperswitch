-- This file should undo anything in `up.sql`
ALTER TABLE payment_link ALTER COLUMN max_age DROP NOT NULL;