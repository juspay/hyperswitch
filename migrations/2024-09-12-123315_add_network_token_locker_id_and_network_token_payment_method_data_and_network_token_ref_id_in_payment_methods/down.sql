-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS network_token_requestor_reference_id;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS network_token_locker_id;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS network_token_payment_method_data;