-- Your SQL goes here
ALTER TABLE gateway_status_map ADD COLUMN error_category VARCHAR(64);

ALTER TABLE gateway_status_map ADD COLUMN error_sub_category VARCHAR(64);
