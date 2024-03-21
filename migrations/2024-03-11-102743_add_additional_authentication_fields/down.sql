-- This file should undo anything in `up.sql`
ALTER TABLE authentication
DROP COLUMN IF EXISTS maximum_supported_version ,
DROP COLUMN IF EXISTS threeds_server_transaction_id ,
DROP COLUMN IF EXISTS cavv ,
DROP COLUMN IF EXISTS authentication_flow_type ,
DROP COLUMN IF EXISTS message_version ,
DROP COLUMN IF EXISTS eci ,
DROP COLUMN IF EXISTS trans_status ,
DROP COLUMN IF EXISTS acquirer_bin ,
DROP COLUMN IF EXISTS acquirer_merchant_id ,
DROP COLUMN IF EXISTS three_ds_method_data ,
DROP COLUMN IF EXISTS three_ds_method_url ,
DROP COLUMN IF EXISTS acs_url ,
DROP COLUMN IF EXISTS challenge_request ,
DROP COLUMN IF EXISTS acs_reference_number ,
DROP COLUMN IF EXISTS acs_trans_id ,
DROP COLUMN IF EXISTS three_dsserver_trans_id ,
DROP COLUMN IF EXISTS acs_signed_content;
