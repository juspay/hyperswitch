ALTER TABLE payment_intent ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

ALTER TABLE payment_attempt ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

ALTER TABLE refund ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

ALTER TABLE connector_response ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

ALTER TABLE reverse_lookup ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

ALTER TABLE address ADD column updated_by VARCHAR(32) NOT NULL DEFAULT 'postgres_only';

