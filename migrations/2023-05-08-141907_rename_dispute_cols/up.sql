-- Your SQL goes here
ALTER TABLE dispute
RENAME COLUMN dispute_created_at TO connector_created_at;

ALTER TABLE dispute
RENAME COLUMN updated_at TO connector_updated_at;
