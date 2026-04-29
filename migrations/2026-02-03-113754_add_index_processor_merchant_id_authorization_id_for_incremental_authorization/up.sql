-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS inc_authz_processor_mid_authz_id_index ON incremental_authorization (processor_merchant_id, authorization_id);