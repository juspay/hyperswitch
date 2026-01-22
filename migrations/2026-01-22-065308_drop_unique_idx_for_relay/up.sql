-- Drop the unique constraint over connector_reference_id to allow multiple capture for relay
DROP INDEX relay_profile_id_connector_reference_id_index;