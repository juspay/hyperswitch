-- Your SQL goes here
CREATE UNIQUE INDEX relay_profile_id_connector_reference_id ON relay (profile_id, connector_reference_id);