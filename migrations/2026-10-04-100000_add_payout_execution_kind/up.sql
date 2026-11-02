-- Apply before deploying code that reads or writes execution_kind.
ALTER TABLE payout_attempt
    ADD COLUMN execution_kind TEXT;
