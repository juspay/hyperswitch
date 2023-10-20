ALTER TABLE payment_attempt
DROP COLUMN surcharge_metadata;


ALTER TABLE payment_intent
ADD surcharge_applicable boolean;