ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS outgoing_webhook_custom_http_headers BYTEA DEFAULT NULL;