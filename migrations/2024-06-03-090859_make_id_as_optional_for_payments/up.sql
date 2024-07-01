-- First drop the primary key of payment_intent
ALTER TABLE payment_intent DROP CONSTRAINT payment_intent_pkey;

-- Create new primary key
ALTER TABLE payment_intent
ADD PRIMARY KEY (payment_id, merchant_id);

-- Make the previous primary key as optional
ALTER TABLE payment_intent
ALTER COLUMN id DROP NOT NULL;

-- Follow the same steps for payment attempt as well
ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;

ALTER TABLE payment_attempt
ADD PRIMARY KEY (attempt_id, merchant_id);

ALTER TABLE payment_attempt
ALTER COLUMN id DROP NOT NULL;
