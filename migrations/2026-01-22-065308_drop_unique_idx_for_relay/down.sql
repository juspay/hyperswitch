-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX relay_profile_id_connector_reference_id_index ON relay USING btree (profile_id, connector_reference_id);