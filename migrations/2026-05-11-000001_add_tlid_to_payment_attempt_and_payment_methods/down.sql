ALTER TABLE payment_attempt DROP COLUMN IF EXISTS network_transaction_link_id;
ALTER TABLE payment_methods DROP COLUMN IF EXISTS network_transaction_link_id;
