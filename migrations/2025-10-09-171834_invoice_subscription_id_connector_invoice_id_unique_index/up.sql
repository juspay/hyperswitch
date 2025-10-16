-- Your SQL goes here
ALTER TABLE invoice ADD CONSTRAINT invoice_subscription_id_connector_invoice_id_unique_index UNIQUE (subscription_id, connector_invoice_id);