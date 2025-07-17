-- Create index on profile_id
CREATE INDEX IF NOT EXISTS merchant_connector_account_profile_id_index 
ON merchant_connector_account (profile_id);
