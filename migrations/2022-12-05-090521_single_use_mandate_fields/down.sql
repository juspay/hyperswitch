-- This file should undo anything in `up.sql`
ALTER TABLE mandate
DROP COLUMN IF EXISTS single_use_amount,
DROP COLUMN IF EXISTS single_use_currency;