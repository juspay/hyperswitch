-- This file should undo anything in `up.sql`
ALTER TABLE payout_attempt
ALTER COLUMN connector TYPE JSONB USING jsonb_build_object (
    'routed_through', connector, 'algorithm', straight_through_algorithm
);

ALTER TABLE payout_attempt DROP COLUMN straight_through_algorithm;

ALTER TABLE payout_attempt ALTER COLUMN connector SET NOT NULL;

DROP type "TransactionType";

ALTER TABLE routing_algorithm DROP COLUMN algorithm_for;