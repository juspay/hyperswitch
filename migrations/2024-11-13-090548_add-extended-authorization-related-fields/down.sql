-- Remove the 'request_extended_authorization' column from the 'payment_intent' table
ALTER TABLE payment_intent
DROP COLUMN request_extended_authorization;

-- Remove the 'request_extended_authorization' and 'extended_authorization_applied' columns from the 'payment_attempt' table
ALTER TABLE payment_attempt
DROP COLUMN request_extended_authorization,
DROP COLUMN extended_authorization_applied;

-- Remove the 'capture_before' column from the 'payment_attempt' table
ALTER TABLE payment_attempt
DROP COLUMN capture_before;

-- Remove the 'always_request_extended_authorization' column from the 'business_profile' table
ALTER TABLE business_profile
DROP COLUMN always_request_extended_authorization;