-- This file should undo anything in `up.sql`
ALTER TABLE payouts 
DROP COLUMN sec_code;

ALTER TABLE payouts 
DROP COLUMN account_type;
