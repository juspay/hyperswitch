-- Your SQL goes here
ALTER TABLE payout_attempt
ALTER COLUMN connector TYPE JSONB USING jsonb_build_object (
    'routed_through', connector, 'algorithm', NULL
);

ALTER TABLE payout_attempt ADD COLUMN routing_info JSONB;

UPDATE payout_attempt
SET
    routing_info = connector -> 'algorithm'
WHERE
    connector ->> 'algorithm' IS NOT NULL;

ALTER TABLE payout_attempt
ALTER COLUMN connector TYPE VARCHAR(64) USING connector ->> 'routed_through';

ALTER TABLE payout_attempt ALTER COLUMN connector DROP NOT NULL;

CREATE type "TransactionType" as ENUM('payment', 'payout');

ALTER TABLE routing_algorithm
ADD COLUMN algorithm_for "TransactionType" DEFAULT 'payment' NOT NULL;

ALTER TABLE routing_algorithm
ALTER COLUMN algorithm_for
DROP DEFAULT;