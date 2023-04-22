-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
ALTER COLUMN connector TYPE JSONB
USING jsonb_build_object(
    'routed_through', connector,
    'algorithm', straight_through_algorithm
);

ALTER TABLE payment_attempt DROP COLUMN straight_through_algorithm;
