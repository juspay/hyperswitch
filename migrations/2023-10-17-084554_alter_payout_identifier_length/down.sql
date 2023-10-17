-- Payout Attempt table
ALTER TABLE
    payout_attempt
ALTER COLUMN
    payout_attempt_id TYPE VARCHAR(64);

ALTER TABLE
    payout_attempt
ALTER COLUMN
    payout_id TYPE VARCHAR(64);

-- Payouts table
ALTER TABLE
    payouts
ALTER COLUMN
    payout_id TYPE VARCHAR(64);

-- Revert mandating profile_id
ALTER TABLE
    payout_attempt
ALTER COLUMN
    profile_id DROP NOT NULL;