-- This file should undo anything in `up.sql`
ALTER TABLE mandate
RENAME COLUMN mandate_amount TO single_use_amount;
ALTER TABLE mandate
RENAME COLUMN mandate_currency TO single_use_currency;
ALTER TABLE mandate
DROP COLUMN IF EXISTS amount_captured,
DROP COLUMN IF EXISTS connector,
DROP COLUMN IF EXISTS connector_mandate_id;