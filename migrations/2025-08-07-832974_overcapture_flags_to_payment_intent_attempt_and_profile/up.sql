ALTER TABLE payment_intent ADD COLUMN request_overcapture BOOLEAN;

ALTER TABLE business_profile
ADD COLUMN always_request_overcapture BOOLEAN;

ALTER TABLE payment_attempt ADD COLUMN overcapture_applied BOOLEAN;