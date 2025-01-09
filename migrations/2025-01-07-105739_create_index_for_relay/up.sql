-- Your SQL goes here
CREATE UNIQUE INDEX relay_profile_id_connector_reference_id_index ON relay (profile_id, connector_reference_id);