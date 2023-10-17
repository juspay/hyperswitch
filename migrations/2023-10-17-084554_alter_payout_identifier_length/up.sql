-- Payout Attempt table
ALTER TABLE
    payout_attempt
ALTER COLUMN
    payout_attempt_id TYPE VARCHAR(128);

ALTER TABLE
    payout_attempt
ALTER COLUMN
    payout_id TYPE VARCHAR(128);

-- Payouts table
ALTER TABLE
    payouts
ALTER COLUMN
    payout_id TYPE VARCHAR(128);

-- Mandate passing profile_id
ALTER TABLE
    payout_attempt
ALTER COLUMN
    profile_id
SET
    NOT NULL;