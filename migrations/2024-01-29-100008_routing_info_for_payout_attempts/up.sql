-- Your SQL goes here
ALTER TABLE
  payout_attempt
ALTER COLUMN
  connector TYPE JSONB USING jsonb_build_object(
    'routed_through',
    connector,
    'algorithm',
    NULL
  );

ALTER TABLE
  payout_attempt
ADD
  COLUMN straight_through_algorithm JSONB;

UPDATE
  payout_attempt
SET
  straight_through_algorithm = connector -> 'algorithm'
WHERE
  connector ->> 'algorithm' IS NOT NULL;

ALTER TABLE
  payout_attempt
ALTER COLUMN
  connector TYPE VARCHAR(64) USING connector ->> 'routed_through';

ALTER TABLE
  payout_attempt
ALTER COLUMN
  connector DROP NOT NULL;