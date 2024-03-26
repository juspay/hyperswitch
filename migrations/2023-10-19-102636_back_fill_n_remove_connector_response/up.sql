-- Your SQL goes here
ALTER TABLE payment_attempt 
ADD COLUMN authentication_data JSON, 
ADD COLUMN encoded_data TEXT;

UPDATE payment_attempt 
SET (authentication_data, encoded_data) = (connector_response.authentication_data, connector_response.encoded_data) 
from connector_response 
where payment_attempt.payment_id = connector_response.payment_id 
    and payment_attempt.attempt_id = connector_response.attempt_id
    and payment_attempt.merchant_id = connector_response.merchant_id;
