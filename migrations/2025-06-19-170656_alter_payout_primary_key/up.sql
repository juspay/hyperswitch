ALTER TABLE payout_attempt DROP CONSTRAINT payout_attempt_pkey;
ALTER TABLE payout_attempt ADD PRIMARY KEY (merchant_id, payout_attempt_id);

ALTER TABLE payouts DROP CONSTRAINT payouts_pkey;
ALTER TABLE payouts ADD PRIMARY KEY (merchant_id, payout_id);

ALTER TABLE payout_attempt ADD COLUMN merchant_order_reference_id VARCHAR(255) NULL;
