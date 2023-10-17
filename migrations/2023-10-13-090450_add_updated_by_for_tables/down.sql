ALTER TABLE payment_intent DROP column updated_by;

ALTER TABLE payment_attempt DROP column updated_by;

ALTER TABLE reverse_lookup DROP column updated_by;

ALTER TABLE address DROP column updated_by;

ALTER TABLE connector_response DROP column updated_by;

ALTER TABLE refund DROP column updated_by;


