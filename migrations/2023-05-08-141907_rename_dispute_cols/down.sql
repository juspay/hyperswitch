ALTER TABLE dispute
RENAME COLUMN connector_created_at TO dispute_created_at;

ALTER TABLE dispute
RENAME COLUMN connector_updated_at TO updated_at;
