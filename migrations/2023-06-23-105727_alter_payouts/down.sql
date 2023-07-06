ALTER TABLE payouts
ADD
    COLUMN payout_method_data JSONB DEFAULT '{}':: JSONB;