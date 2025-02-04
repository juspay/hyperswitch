-- stores the flag send by the merchant during payments-create call
ALTER TABLE payment_intent
ADD COLUMN request_extended_authorization boolean;


ALTER TABLE payment_attempt
-- stores the flag sent to the connector
ADD COLUMN request_extended_authorization boolean;

ALTER TABLE payment_attempt
-- Set to true if extended authentication request was successfully processed by the connector
ADD COLUMN extended_authorization_applied boolean;


ALTER TABLE payment_attempt
-- stores the flag sent to the connector
ADD COLUMN capture_before timestamp;

ALTER TABLE business_profile
-- merchant can configure the default value for request_extended_authorization here
ADD COLUMN always_request_extended_authorization boolean;
