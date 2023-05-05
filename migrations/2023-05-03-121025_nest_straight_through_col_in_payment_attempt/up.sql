-- Your SQL goes here
UPDATE payment_attempt
SET straight_through_algorithm = jsonb_build_object('algorithm', straight_through_algorithm);
