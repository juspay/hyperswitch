-- This file should undo anything in `up.sql`
ALTER TABLE authentication DROP COLUMN three_ds_requestor_trans_id,
DROP COLUMN authentication_url;