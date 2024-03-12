-- This file should undo anything in `up.sql`
ALTER TABLE payout_attempt
ALTER COLUMN connector TYPE JSONB USING jsonb_build_object (
    'routed_through', connector, 'algorithm', routing_info
);

ALTER TABLE payout_attempt DROP COLUMN routing_info;

ALTER TABLE payout_attempt ALTER COLUMN connector SET NOT NULL;


ALTER TABLE routing_algorithm DROP COLUMN algorithm_for;

DROP type "TransactionType";