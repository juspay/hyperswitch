-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS authentication_processor_mid_connector_auth_id_index ON authentication (processor_merchant_id, connector_authentication_id);
