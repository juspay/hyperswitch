-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
ALTER COLUMN id
SET NOT NULL;

ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;

ALTER TABLE payment_attempt
ADD PRIMARY KEY (id);

ALTER TABLE payment_intent
ALTER COLUMN id
SET NOT NULL;

ALTER TABLE payment_intent DROP CONSTRAINT payment_intent_pkey;

ALTER TABLE payment_intent
ADD PRIMARY KEY (id);
